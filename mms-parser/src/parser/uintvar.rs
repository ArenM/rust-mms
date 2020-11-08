use nom::bytes::complete::take;
use nom::IResult;

// TODO: a usize should work in most cases, but there's no reason
// a uintvar can't be bigger than a usize. It would be better to use something like num-bigint
/// A uintvar is a composed of 8 bit sequences, the first bit is 1 when ther are following
/// sequences, and 0 when it is the last byte
pub fn read_uintvar(d: &[u8]) -> IResult<&[u8], u64> {
    let mut nums: Vec<u8> = Vec::new();
    let mut d = d;
    let mut carry = true;

    while carry {
        let (nd, (c, value)) = take_uintvar_byte(d)?;
        carry = c;

        nums.push(value);
        d = nd;
    }

    let value = tally_u7_nums(&nums);

    Ok((d, value))
}

fn tally_u7_nums(nums: &[u8]) -> u64 {
    let mut nums = Vec::from(nums);

    nums.reverse();
    nums.iter()
        .fold((0u64,0), |(acc, places), x| {
            let x = x.clone() as u64;
            (acc + (x << 7 * places), places + 1)
        }).0
}

fn take_uintvar_byte(input: &[u8]) -> IResult<&[u8], (bool, u8)> {
    // The first bit ia a carry bit, the rest are numbers
    let (input, byte) = take(1u8)(input)?;
    let byte = byte[0];

    let carry = byte > 0x7F;
    let number = byte & 0x7F;

    Ok((input, (carry, number)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_1_byte_uintvar() {
        let input: [u8; 1] = [0b00000101];
        let res = read_uintvar(&input);

        let val = res.unwrap().1;
        assert_eq!(val, 0b101u64);
    }

    #[test]
    fn read_2_byte_uintvar() {
        let input: [u8; 2] = [0b10000101, 0b00000001];
        assert_eq!(
            read_uintvar(&input).unwrap().1,
            0b1010000001u64
        );
    }

    #[test]
    fn read_multi_byte_uintvar() {
        let input: [u8; 4] = [0b10000001, 0b10000000, 0b10000000, 0b00000011];
        assert_eq!(
            read_uintvar(&input).unwrap().1,
            0b1000000000000000000011u64
        );
    }

    #[test]
    fn take_uintvar_byte_without_carry() {
        let input: [u8; 1] = [0b00000101];
        let (_new_input, (carry, number)) = take_uintvar_byte(&input).unwrap();

        assert_eq!(carry, false);
        assert_eq!(number, 0b101);
    }

    #[test]
    fn take_uintvar_byte_with_carry() {
        let input: [u8; 1] = [0b11000101];
        let (_new_input, (carry, number)) = take_uintvar_byte(&input).unwrap();

        assert_eq!(carry, true);
        assert_eq!(number, 0b1000101);
    }
}
