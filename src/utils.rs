use crate::FilterError::{self, ConversionError};

pub fn float_to_usize(number: f64, argument: &'static str) -> Result<usize, FilterError> {
    if number.is_finite() {
        let floored = number.floor();
        if 0.0 <= floored && floored <= usize::MAX as f64 {
            Ok(floored as usize)
        } else {
            Err(ConversionError {
                argument,
                value: floored,
            })
        }
    } else {
        Err(ConversionError {
            argument,
            value: number,
        })
    }
}
