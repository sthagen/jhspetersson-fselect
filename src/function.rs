//! Functions for processing values in the query language.
//! This module contains both the regular and aggregate functions used in the query language.

use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::fs::DirEntry;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::time::Duration;

use chrono::Datelike;
use chrono::Local;
use human_time::ToHumanTimeString;
use rand::Rng;
use serde::ser::{Serialize, Serializer};
#[cfg(unix)]
use xattr::FileExt;

use crate::fileinfo::FileInfo;
use crate::util::{capitalize, error_exit, format_date, parse_datetime};
use crate::util::variant::{Variant, VariantType};

macro_rules! functions {
    (
        #[group_order = [$($group_order:literal),*]$(,)?]
        $(#[$enum_attrs:meta])*
        $vis:vis enum $enum_name:ident {
            $(
                #[text = [$($text:literal),*]$(,)? $(data_type = $data_type:literal)?]
                $(@is_aggregate = $is_aggregate:literal)?
                $(@weight = $weight:literal)?
                $(@group = $group:literal)?
                $(@description = $description:literal)?
                $(#[$variant_attrs:meta])*
                $variant:ident
            ),*
            $(,)?
        }
        
    ) => {
        $(#[$enum_attrs])*
        $vis enum $enum_name {
            $(
                $(#[$variant_attrs])*
                $variant,
            )*
        }

        impl FromStr for $enum_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let function = s.to_ascii_lowercase();

                match function.as_str() {
                    $(
                        $(#[$variant_attrs])*
                        $($text)|* => Ok($enum_name::$variant),
                    )*
                    _ => {
                        let err = String::from("Unknown function ") + &function;
                        Err(err)
                    }
                }
            }
        }
        
        impl Display for $enum_name {
            fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
                write!(f, "{:?}", self)
            }
        }

        impl Serialize for $enum_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
        
        impl $enum_name {
            pub fn is_numeric_function(&self) -> bool {
                match self {
                    $(
                        $(#[$variant_attrs])*
                        $enum_name::$variant => {
                            stringify!($($data_type)?) .replace("\"", "") == "numeric"
                        }
                    )*
                }
            }
            
            pub fn is_boolean_function(&self) -> bool {
                match self {
                    $(
                        $(#[$variant_attrs])*
                        $enum_name::$variant => {
                            stringify!($($data_type)?) .replace("\"", "") == "boolean"
                        }
                    )*
                }
            }
            
            pub fn is_aggregate_function(&self) -> bool {
                match self {
                    $(
                        $(#[$variant_attrs])*
                        $enum_name::$variant => {
                            stringify!($($is_aggregate)?) == "true"
                        }
                    )*
                }
            }
            
            pub fn get_weight(&self) -> i32 {
                match self {
                    $(
                        $(#[$variant_attrs])*
                        $enum_name::$variant => {
                            stringify!($($weight)?) .parse().unwrap_or(0)
                        }
                    )*
                }
            }
            
            pub fn get_groups() -> Vec<&'static str> {
                vec![
                    $($group_order),*
                ]
            }

            pub fn get_names_and_descriptions() -> HashMap<&'static str, Vec<(Vec<&'static str>, &'static str)>> {
                let mut map = HashMap::new();

                $(
                    $(#[$variant_attrs])*
                    {
                        if !map.contains_key($($group)?) {
                            map.insert($($group)?, vec![]);
                        }
                        let key = map.get_mut($($group)?).unwrap();
                        key.push((vec![$($text),*], $($description)?));
                    }
                )*

                map
            }
        }
    }
}

functions! {
    #[group_order = ["String", "Japanese string", "Numeric", "Datetime", "Aggregate", "Xattr", "Other"]]
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
    pub enum Function {
        #[text = ["lower", "lowercase", "lcase"]]
        @group = "String"
        @description = "Convert the value to lowercase"
        Lower,
        
        #[text = ["upper", "uppercase", "ucase"]]
        @group = "String"
        @description = "Convert the value to UPPERCASE"
        Upper,
        
        #[text = ["initcap"]]
        @group = "String"
        @description = "Capitalize the first letter of each word (Title Case)"
        InitCap,
        
        #[text = ["length", "len"], data_type = "numeric"]
        @group = "String"
        @description = "Get the length of the string"
        Length,
        
        #[text = ["to_base64", "base64"]]
        @group = "String"
        @description = "Convert the value to base64"
        ToBase64,
        
        #[text = ["from_base64"]]
        @group = "String"
        @description = "Read the value as base64"
        FromBase64,
    
        #[text = ["concat"]]
        @group = "String"
        @description = "Concatenate the value with the arguments"
        Concat,
        
        #[text = ["concat_ws"]]
        @group = "String"
        @description = "Concatenate the arguments, separated by the value"
        ConcatWs,
        
        #[text = ["locate", "position"], data_type = "numeric"]
        @group = "String"
        @description = "Get the position of a substring in the value"
        Locate,
        
        #[text = ["substr", "substring"]]
        @group = "String"
        @description = "Get a substring of the value, from a position and length"
        Substring,
        
        #[text = ["replace"]]
        @group = "String"
        @description = "Replace a substring in the value with another string"
        Replace,
        
        #[text = ["trim"]]
        @group = "String"
        @description = "Trim whitespace from the value"
        Trim,
        
        #[text = ["ltrim"]]
        @group = "String"
        @description = "Trim whitespace from the start of the value"
        LTrim,
        
        #[text = ["rtrim"]]
        @group = "String"
        @description = "Trim whitespace from the end of the value"
        RTrim,
    
        #[text = ["bin"]]
        @group = "Numeric"
        @description = "Get the binary representation of the value"
        Bin,
        
        #[text = ["hex"]]
        @group = "Numeric"
        @description = "Get the hexadecimal representation of the value"
        Hex,
        
        #[text = ["oct"]]
        @group = "Numeric"
        @description = "Get the octal representation of the value"
        Oct,
        
        #[text = ["abs"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the absolute value of the number"
        Abs,
        
        #[text = ["power", "pow"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Raise the value to the power of another value"
        Power,
        
        #[text = ["sqrt"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the square root of the value"
        Sqrt,
        
        #[text = ["log"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the logarithm of the value with a specific base"
        Log,
        
        #[text = ["ln"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the natural logarithm of the value"
        Ln,
        
        #[text = ["exp"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get e raised to the power of the specified number"
        Exp,
        
        #[text = ["least"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the smallest value"
        Least,
        
        #[text = ["greatest"], data_type = "numeric"]
        @group = "Numeric"
        @description = "Get the largest value"
        Greatest,
    
        #[text = ["contains_japanese", "japanese"], data_type = "boolean"]
        @group = "Japanese string"
        @description = "Check if the string contains Japanese characters"
        ContainsJapanese,
        
        #[text = ["contains_hiragana", "hiragana"], data_type = "boolean"]
        @group = "Japanese string"
        @description = "Check if the string contains Hiragana characters"
        ContainsHiragana,
        
        #[text = ["contains_katakana", "katakana"], data_type = "boolean"]
        @group = "Japanese string"
        @description = "Check if the string contains Katakana characters"
        ContainsKatakana,
        
        #[text = ["contains_kana", "kana"], data_type = "boolean"]
        @group = "Japanese string"
        @description = "Check if the string contains Kana characters"
        ContainsKana,
        
        #[text = ["contains_kanji", "kanji"], data_type = "boolean"]
        @group = "Japanese string"
        @description = "Check if the string contains Kanji characters"
        ContainsKanji,
    
        #[text = ["format_size", "format_filesize"]]
        @group = "Other"
        @description = "Format a file size in human-readable format"
        FormatSize,
        
        #[text = ["format_time", "pretty_time"]]
        @group = "Other"
        @description = "Format a time duration in human-readable format"
        FormatTime,
    
        #[text = ["current_date", "cur_date", "curdate"]]
        @weight = 1
        @group = "Datetime"
        @description = "Get the current date"
        CurrentDate,
        
        #[text = ["day"], data_type = "numeric"]
        @group = "Datetime"
        @description = "Get the day from a date"
        Day,
        
        #[text = ["month"], data_type = "numeric"]
        @group = "Datetime"
        @description = "Get the month from a date"
        Month,
        
        #[text = ["year"], data_type = "numeric"]
        @group = "Datetime"
        @description = "Get the year from a date"
        Year,
        
        #[text = ["dayofweek", "dow"], data_type = "numeric"]
        @group = "Datetime"
        @description = "Get the day of the week from a date"
        DayOfWeek,
    
        #[text = ["current_uid"], data_type = "numeric"]
        @weight = 1
        @group = "Other"
        @description = "Get the current user ID"
        #[cfg(all(unix, feature = "users"))]
        CurrentUid,
        
        #[text = ["current_user"]]
        @weight = 1
        @group = "Other"
        @description = "Get the current username"
        #[cfg(all(unix, feature = "users"))]
        CurrentUser,
        
        #[text = ["current_gid"], data_type = "numeric"]
        @weight = 1
        @group = "Other"
        @description = "Get the current group ID"
        #[cfg(all(unix, feature = "users"))]
        CurrentGid,
        
        #[text = ["current_group"]]
        @weight = 1
        @group = "Other"
        @description = "Get the current group name"
        #[cfg(all(unix, feature = "users"))]
        CurrentGroup,
    
        #[text = ["contains"], data_type = "boolean"]
        @weight = 1024
        @group = "Other"
        @description = "Checks if a file contains a substring"
        Contains,
    
        #[text = ["has_xattr"], data_type = "boolean"]
        @weight = 2
        @group = "Xattr"
        @description = "Check if the file has a specific extended attribute"
        #[cfg(unix)]
        HasXattr,
        
        #[text = ["xattr"]]
        @weight = 2
        @group = "Xattr"
        @description = "Get the value of an extended attribute"
        #[cfg(unix)]
        Xattr,
        
        #[text = ["has_capabilities", "has_caps"], data_type = "boolean"]
        @weight = 2
        @group = "Xattr"
        @description = "Check if the file has capabilities (security.capability xattr)"
        #[cfg(target_os = "linux")]
        HasCapabilities,
        
        #[text = ["has_capability", "has_cap"], data_type = "boolean"]
        @weight = 2
        @group = "Xattr"
        @description = "Check if the file has a specific capability (security.capability xattr)"
        #[cfg(target_os = "linux")]
        HasCapability,
    
        #[text = ["coalesce"]]
        @group = "Other"
        @description = "Return the first non-empty value"
        Coalesce,
        
        #[text = ["rand", "random"], data_type = "numeric"]
        @weight = 1
        @group = "Numeric"
        @description = "Gets a random number from 0 to the value, or between two values"
        Random,
    
        #[text = ["min"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the minimum value"
        Min,
        
        #[text = ["max"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the maximum value"
        Max,
        
        #[text = ["avg"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the average value"
        Avg,
        
        #[text = ["sum"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the sum of all values"
        Sum,
        
        #[text = ["count"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the number of values"
        Count,
    
        #[text = ["stddev_pop", "stddev", "std"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the population standard deviation"
        StdDevPop,
        
        #[text = ["stddev_samp"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the sample standard deviation"
        StdDevSamp,
        
        #[text = ["var_pop", "variance"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the population variance"
        VarPop,
        
        #[text = ["var_samp"], data_type = "numeric"]
        @is_aggregate = true
        @group = "Aggregate"
        @description = "Get the sample variance"
        VarSamp,
    }
}

/// Applies a function to a value and returns the result.
/// If no function is provided, the original value is returned.
///
/// Args:
///  function: Optional specification of which function to apply.
///  function_arg: The value to apply the function to.
///  function_args: Additional arguments to the function.
///  entry: Optional directory entry to read the file contents from.
///  file_info: Optional file information to read the file contents from.
///
/// Returns:
///   A variant containing the value computed or the original value if no function is provided.
pub fn get_value(
    function: &Option<Function>,
    function_arg: String,
    function_args: Vec<String>,
    entry: Option<&DirEntry>,
    file_info: &Option<FileInfo>,
) -> Variant {
    //* Refer to the Function enum for a list of available functions and their descriptions
    match function {
        // ===== String functions =====
        Some(Function::Lower) => Variant::from_string(&function_arg.to_lowercase()),
        Some(Function::Upper) => Variant::from_string(&function_arg.to_uppercase()),
        Some(Function::InitCap) => {
            let result = function_arg
                .split_whitespace()
                .map(|s| capitalize(&s.to_lowercase()))
                .collect::<Vec<_>>()
                .join(" ");
            Variant::from_string(&result)
        }
        // Get the length of the string
        Some(Function::Length) => {
            Variant::from_int(function_arg.chars().count() as i64)
        }
        // Convert the value to base64
        Some(Function::ToBase64) => {
            Variant::from_string(&rbase64::encode((function_arg).as_ref()))
        }
        // Read the value as base64
        Some(Function::FromBase64) => {
            Variant::from_string(
                &String::from_utf8_lossy(&rbase64::decode(&function_arg).unwrap_or_default())
                    .to_string(),
            )
        }

        // ===== String manipulation functions =====
        Some(Function::Concat) => {
            Variant::from_string(&(String::from(&function_arg) + &function_args.join("")))
        }
        Some(Function::ConcatWs) => Variant::from_string(&function_args.join(&function_arg)),
        Some(Function::Locate) => {
            let string = String::from(&function_arg);
            let substring = &function_args[0];
            let pos: i32 = match &function_args.get(1) {
                Some(pos) => pos.parse::<i32>().unwrap() - 1,
                _ => 0,
            };
            let string = string.chars().skip(pos as usize).collect::<String>();

            let result = string
                .find(substring)
                .map(|index| index as i64 + pos as i64 + 1)
                .unwrap_or(0);

            Variant::from_int(result)
        },
        Some(Function::Substring) => {
            let string = String::from(&function_arg);

            let mut pos: i32 = match &function_args.is_empty() {
                true => 0,
                false => *&function_args[0].parse::<i32>().unwrap() - 1,
            };

            if pos < 0 {
                let string_length = string.chars().count() as i32;
                pos = string_length - pos.abs() + 1;
            }

            let len = match &function_args.get(1) {
                Some(len) => len.parse::<usize>().unwrap(),
                _ => 0,
            };

            let result = match len > 0 {
                true => string.chars().skip(pos as usize).take(len).collect(),
                false => string.chars().skip(pos as usize).collect(),
            };

            Variant::from_string(&result)
        }
        Some(Function::Replace) => {
            let source = function_arg;
            let from = &function_args[0];
            let to = &function_args[1];

            let result = source.replace(from, to);

            Variant::from_string(&result)
        }
        Some(Function::Trim) => {
            Variant::from_string(&function_arg.trim().to_string())
        }
        Some(Function::LTrim) => {
            Variant::from_string(&function_arg.trim_start().to_string())
        }
        Some(Function::RTrim) => {
            Variant::from_string(&function_arg.trim_end().to_string())
        }

        // ===== Numeric functions =====
        Some(Function::Bin) => match function_arg.parse::<i64>() {
            Ok(val) => Variant::from_string(&format!("{:b}", val)),
            _ => Variant::empty(VariantType::String),
        },
        Some(Function::Hex) => match function_arg.parse::<i64>() {
            Ok(val) => Variant::from_string(&format!("{:x}", val)),
            _ => Variant::empty(VariantType::String),
        },
        Some(Function::Oct) => match function_arg.parse::<i64>() {
            Ok(val) => Variant::from_string(&format!("{:o}", val)),
            _ => Variant::empty(VariantType::String),
        },
        Some(Function::Abs) => match function_arg.parse::<f64>() {
            Ok(val) => Variant::from_float(val.abs()),
            _ => Variant::empty(VariantType::String),
        }
        Some(Function::Power) => {
            match function_arg.parse::<f64>() {
                Ok(val) => {
                    let power = match function_args.first() {
                        Some(power) => power.parse::<f64>().unwrap(),
                        _ => 0.0,
                    };

                    Variant::from_float(val.powf(power))
                }
                _ => Variant::empty(VariantType::String),
            }
        }
        Some(Function::Sqrt) => match function_arg.parse::<f64>() {
            Ok(val) => Variant::from_float(val.sqrt()),
            _ => Variant::empty(VariantType::String),
        },
        Some(Function::Log) => {
            match function_arg.parse::<f64>() {
                Ok(val) => {
                    let base = match function_args.first() {
                        Some(base) => base.parse::<f64>().unwrap(),
                        _ => 10.0,
                    };

                    Variant::from_float(val.log(base))
                }
                _ => Variant::empty(VariantType::String),
            }
        }
        Some(Function::Ln) => match function_arg.parse::<f64>() {
            Ok(val) => Variant::from_float(val.ln()),
            _ => Variant::empty(VariantType::String),
        }
        Some(Function::Exp) => match function_arg.parse::<f64>() {
            Ok(val) => Variant::from_float(val.exp()),
            _ => Variant::empty(VariantType::String),
        }
        Some(Function::Least) => {
            match function_arg.parse::<f64>() {
                Ok(val) => {
                    let mut least = val;
                    for arg in function_args {
                        if let Ok(val) = arg.parse::<f64>() {
                            least = least.min(val);
                        }
                    }

                    Variant::from_float(least)
                }
                _ => Variant::empty(VariantType::String),
            }
        }
        Some(Function::Greatest) => {
            match function_arg.parse::<f64>() {
                Ok(val) => {
                    let mut greatest = val;
                    for arg in function_args {
                        if let Ok(val) = arg.parse::<f64>() {
                            greatest = greatest.max(val);
                        }
                    }

                    Variant::from_float(greatest)
                }
                _ => Variant::empty(VariantType::String),
            }
        }

        // ===== Japanese string functions =====
        Some(Function::ContainsJapanese) => {
            Variant::from_bool(crate::util::japanese::contains_japanese(&function_arg))
        }
        Some(Function::ContainsHiragana) => {
            Variant::from_bool(crate::util::japanese::contains_hiragana(&function_arg))
        }
        Some(Function::ContainsKatakana) => {
            Variant::from_bool(crate::util::japanese::contains_katakana(&function_arg))
        }
        Some(Function::ContainsKana) => {
            Variant::from_bool(crate::util::japanese::contains_kana(&function_arg))
        }
        Some(Function::ContainsKanji) => {
            Variant::from_bool(crate::util::japanese::contains_kanji(&function_arg))
        }

        // ===== Formatting functions =====
        Some(Function::FormatSize) => {
            if function_arg.is_empty() {
                return Variant::empty(VariantType::String);
            }

            if let Ok(size) = function_arg.parse::<u64>() {
                let modifier = match function_args.first() {
                    Some(modifier) => modifier,
                    _ => "",
                };
                let file_size = crate::util::format_filesize(size, modifier);
                return Variant::from_string(&file_size);
            }

            Variant::empty(VariantType::String)
        }
        Some(Function::FormatTime) => {
            if function_arg.is_empty() {
                return Variant::empty(VariantType::String);
            }

            let seconds = function_arg.parse::<u64>().unwrap();
            let formatted = Duration::from_secs(seconds).to_human_time_string();
            Variant::from_string(&formatted)
        }

        // ===== Datetime functions =====
        Some(Function::CurrentDate) => {
            let now = Local::now().date_naive();
            Variant::from_string(&format_date(&now))
        }
        Some(Function::Year) => match parse_datetime(&function_arg) {
            Ok(date) => Variant::from_int(date.0.year() as i64),
            _ => Variant::empty(VariantType::Int),
        },
        Some(Function::Month) => match parse_datetime(&function_arg) {
            Ok(date) => Variant::from_int(date.0.month() as i64),
            _ => Variant::empty(VariantType::Int),
        },
        Some(Function::Day) => match parse_datetime(&function_arg) {
            Ok(date) => Variant::from_int(date.0.day() as i64),
            _ => Variant::empty(VariantType::Int),
        },
        Some(Function::DayOfWeek) => match parse_datetime(&function_arg) {
            Ok(date) => Variant::from_int(date.0.weekday().number_from_sunday() as i64),
            _ => Variant::empty(VariantType::Int),
        },
        
        #[cfg(all(unix, feature = "users"))]
        Some(Function::CurrentUid) => Variant::from_int(uzers::get_current_uid() as i64),
        #[cfg(all(unix, feature = "users"))]
        Some(Function::CurrentUser) => {
            match uzers::get_current_username().and_then(|u| u.into_string().ok()) {
                Some(s) => Variant::from_string(&s),
                None => Variant::empty(VariantType::String),
            }
        }
        #[cfg(all(unix, feature = "users"))]
        Some(Function::CurrentGid) => Variant::from_int(uzers::get_current_gid() as i64),
        #[cfg(all(unix, feature = "users"))]
        Some(Function::CurrentGroup) => {
            match uzers::get_current_groupname().and_then(|u| u.into_string().ok()) {
                Some(s) => Variant::from_string(&s),
                None => Variant::empty(VariantType::String),
            }
        }
        // ===== File functions =====
        Some(Function::Contains) => {
            if file_info.is_some() {
                return Variant::empty(VariantType::Bool);
            }

            if let Some(entry) = entry {
                if let Ok(mut f) = File::open(entry.path()) {
                    let mut contents = String::new();
                    if f.read_to_string(&mut contents).is_ok() {
                        if contents.contains(&function_arg) {
                            return Variant::from_bool(true);
                        } else {
                            return Variant::from_bool(false);
                        }
                    }
                }
            }

            Variant::empty(VariantType::Bool)
        }
        #[cfg(unix)]
        Some(Function::HasXattr) => {
            if let Some(entry) = entry {
                if let Ok(file) = File::open(entry.path()) {
                    if let Ok(xattr) = file.get_xattr(&function_arg) {
                        return Variant::from_bool(xattr.is_some());
                    }
                }
            }

            Variant::empty(VariantType::Bool)
        }
        #[cfg(unix)]
        Some(Function::Xattr) => {
            if let Some(entry) = entry {
                if let Ok(file) = File::open(entry.path()) {
                    if let Ok(Some(xattr)) = file.get_xattr(&function_arg) {
                        if let Ok(value) = String::from_utf8(xattr) {
                            return Variant::from_string(&value);
                        }
                    }
                }
            }

            Variant::empty(VariantType::String)
        }
        #[cfg(target_os = "linux")]
        Some(Function::HasCapabilities) => {
            if let Some(entry) = entry {
                if let Ok(file) = File::open(entry.path()) {
                    if let Ok(caps_xattr) = file.get_xattr("security.capability") {
                        return Variant::from_bool(caps_xattr.is_some());
                    }
                }
            }

            Variant::empty(VariantType::Bool)
        }
        #[cfg(target_os = "linux")]
        Some(Function::HasCapability) => {
            if let Some(entry) = entry {
                if let Ok(file) = File::open(entry.path()) {
                    if let Ok(Some(caps_xattr)) = file.get_xattr("security.capability") {
                        let caps_string = crate::util::capabilities::parse_capabilities(caps_xattr);
                        return Variant::from_bool(caps_string.contains(&function_arg));
                    }
                }
            }

            Variant::empty(VariantType::Bool)
        }
        // ===== Miscellaneous functions =====
        Some(Function::Coalesce) => {
            if !&function_arg.is_empty() {
                return Variant::from_string(&function_arg);
            }

            for arg in function_args {
                if !arg.is_empty() {
                    return Variant::from_string(&arg);
                }
            }

            Variant::empty(VariantType::String)
        }
        Some(Function::Random) => {
            let mut rng = rand::rng();

            if function_arg.is_empty() {
                return Variant::from_int(rng.random_range(0..i64::MAX));
            }

            match function_arg.parse::<i64>() {
                Ok(val) => {
                    if function_args.is_empty() {
                        Variant::from_int(rng.random_range(0..val))
                    } else {
                        let limit = function_args.first().unwrap();
                        match limit.parse::<i64>() {
                            Ok(limit) => Variant::from_int(rng.random_range(val..limit)),
                            _ => error_exit(
                                "Could not parse limit argument of RANDOM function",
                                limit.as_str(),
                            ),
                        }
                    }
                }
                _ => error_exit(
                    "Could not parse an argument of RANDOM function",
                    function_arg.as_str(),
                ),
            }
        }
        // If no function is specified, return the original value
        _ => Variant::empty(VariantType::String),
    }
}

/// Retrieves an aggregated value from a data buffer based on the specified function and key.
///
/// Args:
///   function: Optional specification of which aggregate function to apply.
///   raw_output_buffer: A vector of hashmaps, where each hashmap contains string key-value pairs.
///   buffer_key: The key to look up in each hashmap of the buffer.
///   default_value: An optional default value to return if the function is not specified.
///
/// Returns:
///   A string representation of the aggregate value computed or the default value if no function is provided.
pub fn get_aggregate_value(
    function: &Option<Function>,
    raw_output_buffer: &Vec<HashMap<String, String>>,
    buffer_key: String,
    default_value: &Option<String>,
) -> String {
    //* Refer to the Function enum for a list of available functions and their descriptions
    match function {
        Some(Function::Min) => {
            let min = raw_output_buffer
                .iter()
                .filter_map(|item| item.get(&buffer_key)) // Get the value from the buffer
                .filter_map(|value| value.parse::<i64>().ok()) // Parse the value and filter out errors
                .min()
                .unwrap_or(0); // If no items were found

            min.to_string()
        }
        Some(Function::Max) => {
            let max = raw_output_buffer
                .iter()
                .filter_map(|item| item.get(&buffer_key)) // Get the values from the buffer
                .filter_map(|value| value.parse::<i64>().ok()) // Parse the value and filter out errors
                .max()
                .unwrap_or(0); // If no items were found

            max.to_string()
        }
        Some(Function::Avg) => {
            if raw_output_buffer.is_empty() {
                return String::from("0");
            }

            get_mean(raw_output_buffer, &buffer_key).to_string()
        }
        Some(Function::Sum) => get_buffer_sum(raw_output_buffer, &buffer_key).to_string(),
        Some(Function::Count) => raw_output_buffer.len().to_string(),
        Some(Function::StdDevPop) => {
            if raw_output_buffer.is_empty() {
                return String::new();
            }

            let n = raw_output_buffer.len();
            let variance = get_variance(raw_output_buffer, &buffer_key, n);
            let result = variance.sqrt();

            result.to_string()
        }
        Some(Function::StdDevSamp) => {
            if raw_output_buffer.is_empty() {
                return String::new();
            }

            let size = raw_output_buffer.len();
            let n = if size == 1 { 1 } else { size - 1 };
            let variance = get_variance(raw_output_buffer, &buffer_key, n);
            let result = variance.sqrt();

            result.to_string()
        }
        Some(Function::VarPop) => {
            if raw_output_buffer.is_empty() {
                return String::new();
            }

            let n = raw_output_buffer.len();
            let variance = get_variance(raw_output_buffer, &buffer_key, n);

            variance.to_string()
        }
        Some(Function::VarSamp) => {
            if raw_output_buffer.is_empty() {
                return String::new();
            }

            let size = raw_output_buffer.len();
            let n = if size == 1 { 1 } else { size - 1 };
            let variance = get_variance(raw_output_buffer, &buffer_key, n);

            variance.to_string()
        }

        // If no function is specified, return the default value
        // If no default value was specified, return an empty string
        _ => match &default_value {
            Some(val) => val.to_owned(),
            _ => String::new(),
        },
    }
}

/// Get the variance of all values in the buffer, based on the buffer key.
/// If the value can't be parsed as usize, it will be ignored.
fn get_variance(
    raw_output_buffer: &Vec<HashMap<String, String>>,
    buffer_key: &String,
    n: usize,
) -> f64 {
    let avg = get_mean(raw_output_buffer, buffer_key);

    let mut result: f64 = 0.0;
    for value in raw_output_buffer {
        if let Some(value) = value.get(buffer_key) {
            if let Ok(value) = value.parse::<f64>() {
                result += (avg - value).powi(2) / n as f64;
            }
        }
    }

    result
}

/// Get the mean of all values in the buffer, based on the buffer key.
/// If the value can't be parsed as usize, it will be ignored.
fn get_mean(raw_output_buffer: &Vec<HashMap<String, String>>, buffer_key: &String) -> f64 {
    let sum = get_buffer_sum(raw_output_buffer, buffer_key);
    let size = raw_output_buffer.len();

    (sum / size) as f64
}

/// Get the sum of all values in the buffer, based on the buffer key.
/// If the value can't be parsed as usize, it will be ignored.
fn get_buffer_sum(raw_output_buffer: &Vec<HashMap<String, String>>, buffer_key: &String) -> usize {
    let mut sum = 0;
    for value in raw_output_buffer {
        if let Some(value) = value.get(buffer_key) {
            if let Ok(value) = value.parse::<usize>() {
                sum += value;
            }
        }
    }

    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn function_lower() {
        let function = Function::Lower;
        let function_arg = String::from("HELLO");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello");
    }
    
    #[test]
    fn function_upper() {
        let function = Function::Upper;
        let function_arg = String::from("hello");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "HELLO");
    }
    
    #[test]
    fn function_initcap() {
        let function = Function::InitCap;
        let function_arg = String::from("hello world");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "Hello World");
    }
    
    #[test]
    fn function_length() {
        let function = Function::Length;
        let function_arg = String::from("hello");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 5);
    }
    
    #[test]
    fn function_to_base64() {
        let function = Function::ToBase64;
        let function_arg = String::from("hello");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "aGVsbG8=");
    }
    
    #[test]
    fn function_from_base64() {
        let function = Function::FromBase64;
        let function_arg = String::from("aGVsbG8=");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello");
    }
    
    #[test]
    fn function_concat() {
        let function = Function::Concat;
        let function_arg = String::from("hello");
        let function_args = vec![String::from(" world")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello world");
    }
    
    #[test]
    fn function_concat_ws() {
        let function = Function::ConcatWs;
        let function_arg = String::from(", ");
        let function_args = vec![String::from("hello"), String::from("world")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello, world");
    }
    
    #[test]
    fn function_locate() {
        let function = Function::Locate;
        let function_arg = String::from("hello world");
        let function_args = vec![String::from("world"), String::from("1")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 7);
    }
    
    #[test]
    fn function_substring() {
        let function = Function::Substring;
        let function_arg = String::from("hello world");
        let function_args = vec![String::from("7"), String::from("5")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "world");
    }
    
    #[test]
    fn function_replace() {
        let function = Function::Replace;
        let function_arg = String::from("hello world");
        let function_args = vec![String::from("world"), String::from("Rust")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello Rust");
    }
    
    #[test]
    fn function_trim() {
        let function = Function::Trim;
        let function_arg = String::from("   hello   ");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello");
    }
    
    #[test]
    fn function_ltrim() {
        let function = Function::LTrim;
        let function_arg = String::from("   hello   ");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello   ");
    }
    
    #[test]
    fn function_rtrim() {
        let function = Function::RTrim;
        let function_arg = String::from("   hello   ");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "   hello");
    }
    
    #[test]
    fn function_bin() {
        let function = Function::Bin;
        let function_arg = String::from("10");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "1010");
    }
    
    #[test]
    fn function_hex() {
        let function = Function::Hex;
        let function_arg = String::from("255");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "ff");
    }
    
    #[test]
    fn function_oct() {
        let function = Function::Oct;
        let function_arg = String::from("8");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "10");
    }
    
    #[test]
    fn function_abs() {
        let function = Function::Abs;
        let function_arg = String::from("-10");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 10);
    }
    
    #[test]
    fn function_power() {
        let function = Function::Power;
        let function_arg = String::from("2");
        let function_args = vec![String::from("3")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 8);
    }
    
    #[test]
    fn function_sqrt() {
        let function = Function::Sqrt;
        let function_arg = String::from("16");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 4);
    }
    
    #[test]
    fn function_log() {
        let function = Function::Log;
        let function_arg = String::from("100");
        let function_args = vec![String::from("10")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 2);
    }
    
    #[test]
    fn function_ln() {
        let function = Function::Ln;
        let function_arg = std::f64::consts::E.to_string();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 1);
    }
    
    #[test]
    fn function_exp() {
        let function = Function::Exp;
        let function_arg = String::from("1");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_float(), std::f64::consts::E);
    }

    #[test]
    fn function_least() {
        let function = Function::Least;
        let function_arg = String::from("10");
        let function_args = vec![String::from("20"), String::from("30")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 10);
    }
    
    #[test]
    fn function_greatest() {
        let function = Function::Greatest;
        let function_arg = String::from("10");
        let function_args = vec![String::from("20"), String::from("30")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 30);
    }
    
    #[test]
    fn function_contains_japanese() {
        let function = Function::ContainsJapanese;
        let function_arg = String::from("こんにちは");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_bool(), true);
    }
    
    #[test]
    fn function_contains_hiragana() {
        let function = Function::ContainsHiragana;
        let function_arg = String::from("こんにちは");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_bool(), true);
    }
    
    #[test]
    fn function_contains_katakana() {
        let function = Function::ContainsKatakana;
        let function_arg = String::from("カタカナ");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_bool(), true);
    }
    
    #[test]
    fn function_contains_kana() {
        let function = Function::ContainsKana;
        let function_arg = String::from("カタカナ");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_bool(), true);
    }
    
    #[test]
    fn function_contains_kanji() {
        let function = Function::ContainsKanji;
        let function_arg = String::from("漢字");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_bool(), true);
    }
    
    #[test]
    fn function_format_size() {
        let function = Function::FormatSize;
        let function_arg = String::from("1024");
        let function_args = vec![String::from("%.0 k")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "1 KiB");
    }
    
    #[test]
    fn function_format_time() {
        let function = Function::FormatTime;
        let function_arg = String::from("3600");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "1h");
    }
    
    #[test]
    fn function_current_date() {
        let function = Function::CurrentDate;
        let function_arg = String::new();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), format_date(&Local::now().date_naive()));
    }
    
    #[test]
    fn function_day() {
        let function = Function::Day;
        let function_arg = String::from("2023-10-01");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 1);
    }
    
    #[test]
    fn function_month() {
        let function = Function::Month;
        let function_arg = String::from("2023-10-01");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 10);
    }
    
    #[test]
    fn function_year() {
        let function = Function::Year;
        let function_arg = String::from("2023-10-01");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 2023);
    }
    
    #[test]
    fn function_day_of_week() {
        let function = Function::DayOfWeek;
        let function_arg = String::from("2023-10-01");
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), 1);
    }
    
    #[test]
    #[cfg(all(unix, feature = "users"))]
    fn function_current_uid() {
        let function = Function::CurrentUid;
        let function_arg = String::new();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), uzers::get_current_uid() as i64);
    }
    
    #[test]
    #[cfg(all(unix, feature = "users"))]
    fn function_current_user() {
        let function = Function::CurrentUser;
        let function_arg = String::new();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), uzers::get_current_username().unwrap().to_string_lossy().to_string());
    }
    
    #[test]
    #[cfg(all(unix, feature = "users"))]
    fn function_current_gid() {
        let function = Function::CurrentGid;
        let function_arg = String::new();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_int(), uzers::get_current_gid() as i64);
    }
    
    #[test]
    #[cfg(all(unix, feature = "users"))]
    fn function_current_group() {
        let function = Function::CurrentGroup;
        let function_arg = String::new();
        let function_args = vec![];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), uzers::get_current_groupname().unwrap().to_string_lossy().to_string());
    }
    
    #[test]
    fn function_coalesce() {
        let function = Function::Coalesce;
        let function_arg = String::new();
        let function_args = vec![String::new(), String::from("hello"), String::from("world")];
        let entry = None;
        let file_info = None;

        let result = get_value(&Some(function), function_arg, function_args, entry, &file_info);
        assert_eq!(result.to_string(), "hello");
    }
}