///////////////////////////////////////////////////////////////////
// Search constants exposed for SPSA tuning. With the "tune" feature
// each entry becomes a UCI spin option backed by an atomic; without
// it the accessors compile down to the plain constants.
///////////////////////////////////////////////////////////////////

#[cfg(feature = "tune")]
pub struct Tunable {
    pub name: &'static str,
    pub default: i32,
    pub min: i32,
    pub max: i32,
}

macro_rules! tunables {
    ($($name:ident: $default:expr, $min:expr, $max:expr;)*) => {
        #[cfg(feature = "tune")]
        #[allow(non_upper_case_globals)]
        mod store {
            use std::sync::atomic::AtomicI32;
            $(pub static $name: AtomicI32 = AtomicI32::new($default);)*
        }

        $(
            #[cfg(feature = "tune")]
            #[inline]
            pub fn $name() -> i32 {
                store::$name.load(std::sync::atomic::Ordering::Relaxed)
            }

            #[cfg(not(feature = "tune"))]
            #[inline]
            pub const fn $name() -> i32 {
                $default
            }
        )*

        #[cfg(feature = "tune")]
        pub const OPTIONS: &[Tunable] = &[$(Tunable {
            name: stringify!($name),
            default: $default,
            min: $min,
            max: $max,
        },)*];

        #[cfg(feature = "tune")]
        pub fn contains(name: &str) -> bool {
            OPTIONS.iter().any(|opt| opt.name == name)
        }

        #[cfg(feature = "tune")]
        pub fn set(name: &str, value: i32) -> Result<(), &'static str> {
            match name {
                $(stringify!($name) => {
                    store::$name.store(value.clamp($min, $max), std::sync::atomic::Ordering::Relaxed)
                })*
                _ => return Err("Unknown tunable."),
            }
            Ok(())
        }
    };
}

tunables! {
    futility_margin_multiplier: 100, 40, 200;
    futility_max_depth: 6, 2, 12;
    delta_margin: 200, 80, 400;
    aspiration_window: 12, 5, 50;
    aspiration_growth: 50, 20, 150;
    tt_age_penalty: 8, 2, 24;
    lmr_base_reduction: 11, 0, 50;
    lmr_move_divider: 156, 80, 300;
    rfp_margin_multiplier: 63, 20, 150;
    rfp_improving_margin: 30, 0, 100;
    singular_margin: 200, 50, 400;
    null_depth_divider: 200, 120, 400;
}
