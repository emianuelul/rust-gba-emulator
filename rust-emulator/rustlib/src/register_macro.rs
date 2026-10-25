pub trait Field {
    fn from_bits(v: u32) -> Self;
    fn to_bits(self) -> u32;
}

impl Field for bool {
    fn from_bits(v: u32) -> Self {
        v == 1
    }
    fn to_bits(self) -> u32 {
        self as u32
    }
}

impl Field for u8 {
    fn from_bits(v: u32) -> Self {
        v as u8
    }
    fn to_bits(self) -> u32 {
        self as u32
    }
}

impl Field for u16 {
    fn from_bits(v: u32) -> Self {
        v as u16
    }
    fn to_bits(self) -> u32 {
        self as u32
    }
}

impl Field for u32 {
    fn from_bits(v: u32) -> Self {
        v
    }
    fn to_bits(self) -> u32 {
        self
    }
}

#[macro_export]
macro_rules! register {
    ($name:ident : $raw:ty {$($field_name:ident : $field_type:ty => $tuple:expr),* $(,)?}) => {
        pub struct $name {
            $(pub $field_name: $field_type),*
        }

        impl From<$raw> for $name {
            fn from(raw: $raw) -> $name {
                let raw = raw as u32;
                $name {
                    $($field_name: $crate::register_macro::Field::from_bits($crate::utility_fns::get_field_bits(raw, ($tuple.0, $tuple.1)))),*
                }
            }
        }

        impl From<$name> for $raw {
            fn from(r: $name) -> $raw {
                let mut raw: u32 = 0;
                $(raw |= $crate::register_macro::Field::to_bits(r.$field_name) << $tuple.0;)*
                    raw as $raw
            }
        }
    };
}
