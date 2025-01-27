/// this file implements any conversion from Robject to polars::Series
/// most other R to polars conversion uses the module only pub function robjname2series()

use polars::prelude as pl;
use extendr_api::prelude::*;
use polars::prelude::NamedFrom;
use polars::prelude::IntoSeries;

// Internal tree structure to contain Series of fully parsed nested Robject.
// It is easier to resolve concatenated datatype after all elements have been parsed
// because empty lists have no type in R, but the corrosponding polars type must be known before
// concatenation.
#[derive(Debug)]
enum SeriesTree {
    Series(pl::Series),         // an R object likely some vector was converted into a plain Series
    SeriesVec(Vec<SeriesTree>), // an R object was converted into list of Series'
    SeriesEmptyVec,             // likely an R NULL or list() delayed conversion as corrosponding polars is yet given
}

// Main module function: Convert any potentially nested R object handled in three steps
pub fn robjname2series(x: &Robj, name: &str) -> pl::PolarsResult<pl::Series> {
    // 1 parse any (potentially) R structure, into a tree of Series, boubble any parse error
    let st = recursive_robjname2series_tree(x, name)?;

    // 2 search for first leaf dtype, returns None for empty list or lists of empty lists and so on ...
    let first_leaf_dtype = find_first_leaf_datatype(&st);

    // 3 concat SeriesTree into one Series, boubble any type mismatch error
    concat_series_tree(st, &first_leaf_dtype, name)
}

// this function walks the SeriesTree to find the first leaf DataType
fn find_first_leaf_datatype(st: &SeriesTree) -> Option<pl::DataType> {
    match st {
        SeriesTree::Series(s) => Some(s.dtype().clone()), //an actual leaf type found, return as the answer
        SeriesTree::SeriesEmptyVec => None, //no type to be found here in this empty list return None from here
        SeriesTree::SeriesVec(sv) => sv //looking deeper in nested structure
            .iter()
            .map(|inner_st| find_first_leaf_datatype(inner_st))
            .filter(|x| x.is_some())
            .next() //get the first None answer
            .flatten(), //alias outer option (empty list) with inner option (inner empty list)
    }
}

// convert any Robj into a SeriesTree, or a nested SeriesTree if nested Robject
fn recursive_robjname2series_tree(x: &Robj, name: &str) -> pl::PolarsResult<SeriesTree> {
    let rtype = x.rtype();

    // handle any supported Robj
    match rtype {
        Rtype::Doubles => {
            let rdouble: Doubles = x.try_into().expect("as matched");
            if rdouble.no_na().is_true() {
                Ok(SeriesTree::Series(pl::Series::new(
                    name,
                    x.as_real_slice().unwrap(),
                )))
            } else {
                let mut s: pl::Series = rdouble//convert R NAs to rust options
                    .iter()
                    .map(|x| if x.is_na() { None } else { Some(x.0) })
                    .collect();
                s.rename(name);
                Ok(SeriesTree::Series(s))
            }
        }

        Rtype::Strings => Ok(SeriesTree::Series(robj_to_utf8_series(x.try_into().expect("as matched"), name))),

        Rtype::Logicals => {
            let logicals: Logicals = x.try_into().unwrap();
            let s: Vec<Option<bool>> = logicals
                .iter()
                .map(|x| if x.is_na() { None } else { Some(x.is_true()) })
                .collect();
            Ok(SeriesTree::Series(pl::Series::new(name, s)))
        }

        Rtype::Integers if x.inherits("factor") => Ok(SeriesTree::Series(
            robj_to_utf8_series(x.as_character_factor().try_into().expect("as_character_factor() enforces same type"), name)
                .cast(&pl::DataType::Categorical(None))
                .expect("as matched"),
        )),

        Rtype::Integers => {
            let rints = x.as_integers().expect("as matched");
            let s = if rints.no_na().is_true() {
                pl::Series::new(name, x.as_integer_slice().expect("as matched"))
            } else {
                //convert R NAs to rust options
                let mut s: pl::Series = rints
                    .iter()
                    .map(|x| if x.is_na() { None } else { Some(x.0) })
                    .collect();
                s.rename(name);
                s
            };
            Ok(SeriesTree::Series(s))
        },

        Rtype::Null => Ok(SeriesTree::SeriesEmptyVec), // flag NULL with this enum, to resolve polars type later

        Rtype::List => { // Recusively handle elements of list
            let result_series_vec: pl::PolarsResult<Vec<SeriesTree>> = x
                .as_list()
                .unwrap()
                .iter()
                .map(|(name, robj)| recursive_robjname2series_tree(&robj, name))
                .collect();
            result_series_vec.map(|vst| {
                if vst.len() == 0 {
                    SeriesTree::SeriesEmptyVec  // flag empty list() with this enum, to resolve polars type later
                } else {
                    SeriesTree::SeriesVec(vst)
                }
            })
        }

        _ => Err(pl::PolarsError::NotFound(polars::error::ErrString::Owned(
            format!("new series from rtype {:?} is not supported (yet)", rtype),
        ))),
    }
}

// consume nested SeriesTree and return concatenated Series or an appropriate Error
fn concat_series_tree(
    st: SeriesTree,
    leaf_dtype: &Option<pl::DataType>,
    name: &str,
) -> pl::PolarsResult<pl::Series> {
    match st {
        SeriesTree::Series(s) => Ok(s), // SeriesTree is just a regular Series, return as is
        SeriesTree::SeriesEmptyVec => { // Create Series of empty array and cast to the found leaf_dtype.
            let empty_list_series = pl::Series::new(name, [0f64; 0]).to_list()?.slice(0, 0);
            let s = empty_list_series.into_series();
            if let Some(leaf_dt_ref) = leaf_dtype {
                s.cast(leaf_dt_ref)
            } else {
                Ok(s) //use float as default DataType for empty lists of lists all the way down
            }
        },
        SeriesTree::SeriesVec(sv) if sv.len() == 0 => unreachable!(
            "internal error: A series tree was built with a literal empty vector, instead of using the SeriesEmptyVec flag"
        ),
        SeriesTree::SeriesVec(sv) => {
            
            // concat any deeper nested parts of SeriesTree
            let series_vec_result: pl::PolarsResult<Vec<pl::Series>> = sv
                .into_iter()
                .map(|inner_st| concat_series_tree(inner_st, leaf_dtype, ""))
                .collect();

            // boubble any errors
            let series_vec = series_vec_result?;

            // check for any type mismatch to avoid polars panics
            let mut s_iter = series_vec.iter();
            let first_s = s_iter.next();
            for s_ref in s_iter {
                if s_ref.dtype() != first_s.expect("could not loop if none first_s").dtype() {
                    Err(pl::PolarsError::SchemaMisMatch(polars::error::ErrString::Owned(format!(
                        "When building series from R list; some parsed sub-elements did not match: One element was {} and another was {}",
                        first_s.expect("dont worry about it").dtype(),s_ref.dtype()
                    ))))?;
                }
            }

            // use polars new method to concat concatenated series
            Ok(pl::Series::new(name, series_vec))
        }
    }
}

//handle R character/strings to utf8
fn robj_to_utf8_series(rstrings: Strings, name: &str) -> pl::Series {
    if rstrings.no_na().is_true() {
        pl::Series::new(name, rstrings.as_robj().as_str_vector().unwrap())
    } else {
        //convert R NAs to rust options
        let s: Vec<Option<&str>> = rstrings
            .iter()
            .map(|x| if x.is_na() { None } else { Some(x.as_str()) })
            .collect();
        let s = pl::Series::new(name, s);
        s
    }
}
