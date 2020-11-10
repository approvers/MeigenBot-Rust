/// エラーを表すためのenum作成マクロ。
/// Trailing comma 対応
/// # 書式
///
/// ```
/// make_error_enum! {
///     enum_name;
///     variant_name generator_func_name(format_args) => "format_template",
///     // バリアント定義は何個でも書ける
/// }
/// ```

#[macro_export]
macro_rules! make_error_enum {
    ($enum_name:ident; $($variant:ident $func_name:ident($($($vars:ident),+ $(,)?)?) => $format:expr),+ $(,)?) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($variant(String),)+
        }

        impl $enum_name {
            $ (
                pub fn $func_name($($($vars: impl std::fmt::Display,)*)?) -> $enum_name {
                    $enum_name::$variant(format!($format, $($($vars),+)?))
                }
            )+
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($enum_name::$variant(text) => write!(f, "{}", text),)+
                }
            }
        }
    };
}
