//! Primitive operations shared by constant folding and the VM.

use crate::program::BinOp;
use crate::error::VmError;
use crate::program::Value;

/// Maximum size of a single text/bytes/list value in bytes.
pub const MAX_VALUE_BYTES: usize = 65_536;
/// Maximum number of elements of a list held in memory.
pub const MAX_LIST_LEN: usize = 4_096;

fn int(v: &Value) -> Result<i128, VmError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(VmError::Type(format!("expected int, got {other:?}"))),
    }
}

/// Applies a non-short-circuit binary operator.
pub fn binary(op: BinOp, l: &Value, r: &Value) -> Result<Value, VmError> {
    Ok(match op {
        BinOp::Add => match (l, r) {
            (Value::Int(a), Value::Int(b)) => Value::Int(a.checked_add(*b).ok_or(VmError::Overflow)?),
            (Value::Text(a), Value::Text(b)) => {
                if a.len() + b.len() > MAX_VALUE_BYTES {
                    return Err(VmError::TooLarge);
                }
                Value::Text(format!("{a}{b}"))
            }
            (Value::Bytes(a), Value::Bytes(b)) => {
                if a.len() + b.len() > MAX_VALUE_BYTES {
                    return Err(VmError::TooLarge);
                }
                let mut v = a.clone();
                v.extend_from_slice(b);
                Value::Bytes(v)
            }
            _ => return Err(VmError::Type("bad operands for +".into())),
        },
        BinOp::Sub => Value::Int(int(l)?.checked_sub(int(r)?).ok_or(VmError::Overflow)?),
        BinOp::Mul => Value::Int(int(l)?.checked_mul(int(r)?).ok_or(VmError::Overflow)?),
        BinOp::Div => {
            let b = int(r)?;
            if b == 0 {
                return Err(VmError::DivisionByZero);
            }
            Value::Int(int(l)?.checked_div(b).ok_or(VmError::Overflow)?)
        }
        BinOp::Rem => {
            let b = int(r)?;
            if b == 0 {
                return Err(VmError::DivisionByZero);
            }
            Value::Int(int(l)?.checked_rem(b).ok_or(VmError::Overflow)?)
        }
        BinOp::Eq => Value::Bool(l == r),
        BinOp::Ne => Value::Bool(l != r),
        BinOp::Lt => Value::Bool(int(l)? < int(r)?),
        BinOp::Le => Value::Bool(int(l)? <= int(r)?),
        BinOp::Gt => Value::Bool(int(l)? > int(r)?),
        BinOp::Ge => Value::Bool(int(l)? >= int(r)?),
        BinOp::And | BinOp::Or => {
            let (Value::Bool(a), Value::Bool(b)) = (l, r) else {
                return Err(VmError::Type("bad operands for and/or".into()));
            };
            Value::Bool(if op == BinOp::And { *a && *b } else { *a || *b })
        }
    })
}

/// ⌊a × b ÷ c⌋ rounded toward zero, exact for every i128 input whose result fits.
pub fn mul_div(a: i128, b: i128, c: i128) -> Result<i128, VmError> {
    if c == 0 {
        return Err(VmError::DivisionByZero);
    }
    let negative = (a < 0) ^ (b < 0) ^ (c < 0);
    let (hi, lo) = mul_u128(a.unsigned_abs(), b.unsigned_abs());
    let q = div_u256(hi, lo, c.unsigned_abs()).ok_or(VmError::Overflow)?;
    if negative {
        if q > i128::MAX as u128 + 1 {
            return Err(VmError::Overflow);
        }
        Ok((q as i128).wrapping_neg())
    } else {
        i128::try_from(q).map_err(|_| VmError::Overflow)
    }
}

fn mul_u128(a: u128, b: u128) -> (u128, u128) {
    let mask = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & mask);
    let (b1, b0) = (b >> 64, b & mask);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & mask) + (p10 & mask);
    let lo = (p00 & mask) | ((mid & mask) << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
}

/// (hi, lo) ÷ d, `None` if the quotient does not fit in u128.
fn div_u256(hi: u128, lo: u128, d: u128) -> Option<u128> {
    if hi >= d {
        return None;
    }
    let mut rem = hi;
    let mut q: u128 = 0;
    for i in (0..128).rev() {
        let carry = rem >> 127;
        rem = (rem << 1) | ((lo >> i) & 1);
        q <<= 1;
        if carry == 1 || rem >= d {
            rem = rem.wrapping_sub(d);
            q |= 1;
        }
    }
    Some(q)
}

/// ⌊√x⌋.
pub fn isqrt(x: i128) -> Result<i128, VmError> {
    if x < 0 {
        return Err(VmError::BadArguments("isqrt() of a negative number".into()));
    }
    let x = x as u128;
    if x < 2 {
        return Ok(x as i128);
    }
    let mut r = 1u128 << ((128 - x.leading_zeros()).div_ceil(2));
    loop {
        let next = (r + x / r) / 2;
        if next >= r {
            break;
        }
        r = next;
    }
    while r * r > x {
        r -= 1;
    }
    Ok(r as i128)
}

/// Checked integer power.
pub fn pow(base: i128, exp: i128) -> Result<i128, VmError> {
    if exp < 0 {
        return Err(VmError::BadArguments("pow() needs a non-negative exponent".into()));
    }
    match base {
        0 | 1 => return Ok(if exp == 0 { 1 } else { base }),
        -1 => return Ok(if exp % 2 == 0 { 1 } else { -1 }),
        _ => {}
    }
    let e = u32::try_from(exp).map_err(|_| VmError::Overflow)?;
    base.checked_pow(e).ok_or(VmError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_is_checked() {
        assert_eq!(binary(BinOp::Add, &Value::Int(2), &Value::Int(3)).unwrap(), Value::Int(5));
        assert_eq!(binary(BinOp::Add, &Value::Int(i128::MAX), &Value::Int(1)), Err(VmError::Overflow));
        assert_eq!(binary(BinOp::Div, &Value::Int(1), &Value::Int(0)), Err(VmError::DivisionByZero));
        assert_eq!(binary(BinOp::Div, &Value::Int(i128::MIN), &Value::Int(-1)), Err(VmError::Overflow));
        assert_eq!(binary(BinOp::Rem, &Value::Int(-7), &Value::Int(2)).unwrap(), Value::Int(-1));
        assert_eq!(binary(BinOp::Add, &Value::Text("a".into()), &Value::Text("b".into())).unwrap(), Value::Text("ab".into()));
    }

    #[test]
    fn math_helpers() {
        assert_eq!(mul_div(i128::MAX, i128::MAX, i128::MAX).unwrap(), i128::MAX);
        assert_eq!(mul_div(10, 3, 4).unwrap(), 7);
        assert_eq!(mul_div(-10, 3, 4).unwrap(), -7);
        assert_eq!(mul_div(i128::MAX, 2, 1), Err(VmError::Overflow));
        assert_eq!(mul_div(1, 1, 0), Err(VmError::DivisionByZero));
        assert_eq!(mul_div(i128::MIN, 1, 1).unwrap(), i128::MIN);
        assert_eq!(mul_div(1 << 100, 1 << 100, 1 << 90).unwrap(), 1 << 110);
        for x in [0i128, 1, 2, 3, 4, 15, 16, 17, 1 << 62, (1 << 62) - 1, i128::MAX] {
            let r = isqrt(x).unwrap();
            assert!(r * r <= x && (r + 1).checked_mul(r + 1).is_none_or(|s| s > x), "isqrt({x})");
        }
        assert!(isqrt(-1).is_err());
        assert_eq!(pow(2, 10).unwrap(), 1024);
        assert_eq!(pow(-1, 1 << 100).unwrap(), 1);
        assert_eq!(pow(10, 39), Err(VmError::Overflow));
        assert!(pow(2, -1).is_err());
    }
}
