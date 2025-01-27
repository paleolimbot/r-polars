use crate::utils::r_result_list;
use crate::utils::wrappers::Wrap;
use extendr_api::prelude::*;
use polars::prelude::{self as pl};
use polars_core::prelude::QuantileInterpolOptions;
//expose polars DateType in R
#[extendr]
#[derive(Debug, Clone, PartialEq)]
pub struct DataType(pub pl::DataType);

#[extendr]
impl DataType {
    pub fn new(s: &str) -> DataType {
        //, inner: Nullable<&DataType>
        //let inner = Box::new(null_to_opt(inner).map_or(pl::DataType::Null, |x| x.0.clone()));

        let pl_datatype = match s {
            "Boolean" | "logical" => pl::DataType::Boolean,
            "UInt8" | "uinteger8" => pl::DataType::UInt8,
            "UInt16" | "uinteger16" => pl::DataType::UInt16,
            "UInt32" | "uinteger32" => pl::DataType::UInt32,
            "UInt64" | "uinteger64" => pl::DataType::UInt64,
            "Int8" | "integer8" => pl::DataType::Int8,
            "Int16" | "integer16" => pl::DataType::Int16,
            "Int32" | "integer32" | "integer" => pl::DataType::Int32,
            "Int64" | "integer64" => pl::DataType::Int64,
            "Float32" | "float32" | "double" => pl::DataType::Float32,
            "Float64" | "float64" => pl::DataType::Float64,

            "Utf8" | "character" => pl::DataType::Utf8,
            "Binary" | "binary" => pl::DataType::Binary,
            "Date" | "date" => pl::DataType::Date,
            "Time" | "time" => pl::DataType::Time,
            "Null" | "null" => pl::DataType::Null,
            "Categorical" | "factor" => pl::DataType::Categorical(None),
            "Unknown" | "unknown" => pl::DataType::Unknown,

            _ => panic!("data type not recgnized"),
        };
        DataType(pl_datatype)
    }

    pub fn new_datetime() -> DataType {
        todo!("datetime not implemented")
    }

    pub fn new_duration() -> DataType {
        todo!("duration not implemented")
    }

    pub fn new_list(inner: &DataType) -> DataType {
        DataType(pl::DataType::List(Box::new(inner.0.clone())))
    }

    pub fn new_object() -> DataType {
        todo!("object not implemented")
    }

    pub fn new_struct() -> DataType {
        todo!("struct not implemented")
    }

    pub fn get_all_simple_type_names() -> Vec<String> {
        vec![
            "Boolean".into(),
            "UInt8".into(),
            "UInt16".into(),
            "UInt32".into(),
            "UInt64".into(),
            "Int8".into(),
            "Int16".into(),
            "Int32".into(),
            "Int64".into(),
            "Float32".into(),
            "Float64".into(),
            "Utf8".into(),
            "Binary".into(),
            "Date".into(),
            "Time".into(),
            "Null".into(),
            "Categorical".into(),
            "Unknown".into(),
        ]
    }

    pub fn print(&self) {
        rprintln!("{:#?}", self.0);
    }

    pub fn eq(&self, other: &DataType) -> bool {
        self.0.eq(&other.0)
    }

    pub fn ne(&self, other: &DataType) -> bool {
        self.0.ne(&other.0)
    }
}

impl From<DataType> for pl::DataType {
    fn from(x: DataType) -> Self {
        x.0
    }
}

//struct for building a vector of optional named datatype,
//if all named will become a schema and passed to polars_io.csv.csvread.with_dtypes
//if any names are missing will become slice of dtypes and passed to polars_io.csv.csvread.with_dtypes_slice
//zero length vector will neither trigger with_dtypes() or with_dtypes_slice() method calls
#[derive(Debug, Clone)]
#[extendr]
pub struct DataTypeVector(pub Vec<(Option<String>, pl::DataType)>);

#[extendr]
impl DataTypeVector {
    pub fn new() -> Self {
        DataTypeVector(Vec::new())
    }

    pub fn push(&mut self, colname: Nullable<String>, datatype: &DataType) {
        self.0.push((Wrap(colname).into(), datatype.clone().into()));
    }

    pub fn print(&self) {
        rprintln!("{:#?}", self.0);
    }

    pub fn from_rlist(list: List) -> List {
        let mut dtv = DataTypeVector(Vec::with_capacity(list.len()));

        let result: std::result::Result<(), String> = list
            .iter()
            .map(|(name, robj)| -> std::result::Result<(), String> {
                if !robj.inherits("DataType") || robj.rtype() != extendr_api::Rtype::ExternalPtr {
                    return Err("Internal error: Object is not a DataType".into());
                }
                //safety checks class and type before conversion
                let dt: DataType = unsafe { &mut *robj.external_ptr_addr::<DataType>() }.clone();
                let name = extendr_api::Nullable::NotNull(name.to_string());
                dtv.push(name, &dt);
                Ok(())
            })
            .collect();

        r_result_list(result.map(|_| dtv))
    }
}

impl DataTypeVector {
    pub fn dtv_to_vec(&self) -> Vec<pl::DataType> {
        let v: Vec<_> = self.0.iter().map(|(_, dt)| dt.clone()).collect();
        v
    }
}

pub fn new_join_type(s: &str) -> pl::JoinType {
    match s {
        "cross" => pl::JoinType::Cross,
        "inner" => pl::JoinType::Inner,
        "left" => pl::JoinType::Left,
        "outer" => pl::JoinType::Outer,
        "semi" => pl::JoinType::Semi,
        "anti" => pl::JoinType::Anti,
        _ => panic!("rpolars internal error: jointype not recognized"),
    }
}

pub fn new_quantile_interpolation_option(
    s: &str,
) -> std::result::Result<QuantileInterpolOptions, String> {
    use pl::QuantileInterpolOptions::*;
    match s {
        "nearest" => Ok(Nearest),
        "higher" => Ok(Higher),
        "lower" => Ok(Lower),
        "midpoint" => Ok(Midpoint),
        "linear" => Ok(Linear),
        _ => Err(format!("interpolation choice: [{}] is not any of 'nearest', 'higher', 'lower', 'midpoint', 'linear'",s))
    }
}

pub fn new_closed_window(s: &str) -> std::result::Result<pl::ClosedWindow, String> {
    use pl::ClosedWindow as CW;
    match s {
        "both" => Ok(CW::Both),
        "left" => Ok(CW::Left),
        "none" => Ok(CW::None),
        "right" => Ok(CW::Right),
        _ => Err(format!(
            "ClosedWindow choice: [{}] is not any of 'both', 'left', 'none' or 'right'",
            s
        )),
    }
}

pub fn new_null_behavior(
    s: &str,
) -> std::result::Result<polars::series::ops::NullBehavior, String> {
    use polars::series::ops::NullBehavior as NB;
    match s {
        "ignore" => Ok(NB::Ignore),
        "drop" => Ok(NB::Drop),

        _ => Err(format!(
            "NullBehavior choice: [{}] is not any of 'drop' or 'ignore'",
            s
        )),
    }
}

pub fn new_rank_method(s: &str) -> std::result::Result<pl::RankMethod, String> {
    use pl::RankMethod as RM;
    let s_low = s.to_lowercase();
    match s_low.as_str() {
        "average" => Ok(RM::Average),
        "dense" => Ok(RM::Dense),
        "max" => Ok(RM::Max),
        "min" => Ok(RM::Min),
        "ordinal" => Ok(RM::Ordinal),
        "random" => Ok(RM::Random),
        _ => Err(format!(
            "RankMethod choice: [{}] is not any 'average','dense', 'min', 'max', 'ordinal', 'random'",
            s_low.as_str()
        )),
    }
}

pub fn literal_to_any_value(
    litval: pl::LiteralValue,
) -> std::result::Result<pl::AnyValue<'static>, String> {
    use pl::AnyValue as av;
    use pl::LiteralValue as lv;
    use smartstring::alias::String as SString;
    match litval {
        lv::Boolean(x) => Ok(av::Boolean(x)),
        //lv::DateTime(datetime, unit) => Ok(av::Datetime(datetime, unit, &None)), #check how to convert
        //lv::Duration(duration, unit) => Ok(av::Duration(duration, unit)), #check how to convert
        lv::Float32(x) => Ok(av::Float32(x)),
        lv::Float64(x) => Ok(av::Float64(x)),
        lv::Int16(x) => Ok(av::Int16(x)),
        lv::Int32(x) => Ok(av::Int32(x)),
        lv::Int64(x) => Ok(av::Int64(x)),
        lv::Int8(x) => Ok(av::Int8(x)),
        lv::Null => Ok(av::Null),
        // lv::Range {
        //     low,
        //     high,
        //     data_type,
        // } => Ok(av::(low, high, data_type)),
        //lv::Series(s) => no counter part
        lv::UInt16(x) => Ok(av::UInt16(x)),
        lv::UInt32(x) => Ok(av::UInt32(x)),
        lv::UInt64(x) => Ok(av::UInt64(x)),
        lv::UInt8(x) => Ok(av::UInt8(x)),
        // lv::Utf8(x) => Ok(av::Utf8(x.as_str())),
        lv::Utf8(x) => {
            let mut s = SString::new();

            s.push_str(x.as_str());
            Ok(av::Utf8Owned(s))
        }
        x => Err(format!("cannot convert LiteralValue {:?} to AnyValue", x)),
    }
}

// // this function seemed nifty as it would be possible to evalute casted literals into a anyvalue
// // that would have made it easy from R to express anyvalue as a casted literal.
// // but could not return the AnyValue due to lifetime stuff
// pub fn expr_to_any_value(e: pl::Expr) -> std::result::Result<pl::AnyValue<'static>, String> {
//     use pl::*;
//     let x = Ok(pl::DataFrame::default()
//         .lazy()
//         .select(&[e])
//         .collect()
//         .map_err(|err| err.to_string())?
//         .iter()
//         .next()
//         .ok_or_else(|| String::from("expr made now value"))?
//         .iter()
//         .next()
//         .ok_or_else(|| String::from("expr made now value"))?
//         );
//     x
// }

pub fn new_interpolation_method(s: &str) -> std::result::Result<pl::InterpolationMethod, String> {
    use pl::InterpolationMethod as IM;
    match s {
        "linear" => Ok(IM::Linear),
        "nearest" => Ok(IM::Nearest),

        _ => Err(format!(
            "InterpolationMethod choice: [{}] is not any of 'linear' or 'nearest'",
            s
        )),
    }
}

extendr_module! {
    mod rdatatype;
    impl DataType;
    impl DataTypeVector;
}
