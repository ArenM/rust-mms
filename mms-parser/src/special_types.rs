use nom::bits::complete::take;
use nom::IResult;

// TODO: a usize should work in most cases, but there's no reason
// a unitvar can't be bigger than a usize. It would be better to use something like num-bigint
/// A unitvar is a composed of 8 bit sequences, the first bit is 1 when ther are following
/// sequences, and 0 when it is the last byte
pub fn uintvar(d: &[u8]) -> IResult<&[u8], usize> {
    let mut nums: Vec<u8> = Vec::new();
    let mut d = d;

    loop {
        let (nd, (carry, value)) = take_uintvar_byte(d)?;

        nums.push(value);
        d = nd;

        println!("Carry: {:?}, Nums: {:?}", carry, nums);
        if carry == 0 {
            break;
        };
    }

    let value = tally_u7_nums(&nums);
    println!("Value: {:?}", value);

    Ok((d, value))
}

fn tally_u7_nums(nums: &[u8]) -> usize {
    let mut nums = Vec::from(nums);

    nums.reverse();
    nums.iter()
        .fold((0, 0), |(acc, places), x| {
            let x: usize = *x as usize;
            let shift = 2usize.pow(7 * places);
            println!("x, shift, places: {:?}, {:?}, {:?}", x, shift, places);
            (acc + x * shift, places + 1)
        })
        .0
}

fn tuple_to_u8s(
    i: ((&[u8], impl std::any::Any), nom::error::ErrorKind),
) -> (&[u8], nom::error::ErrorKind) {
    ((i.0).0, i.1)
}

fn take_uintvar_byte(input: &[u8]) -> IResult<&[u8], (u8, u8)> {
    // println!("input: {:?}", input);
    let ((_, _remainder), carry): ((_, _), u8) = match take(1u8)((input, 0usize)) {
        Ok(v) => Ok(v),
        Err(e) => Err(e.map(tuple_to_u8s)),
    }?;

    let ((input, _remainder), number): ((_, _), u8) = match take(7u8)((input, 1usize)) {
        Ok(v) => Ok(v),
        Err(e) => Err(e.map(tuple_to_u8s)),
    }?;

    // let (passthrough, _) = nom::bytes::complete::take(1usize)(input)?;

    Ok((input, (carry, number)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_1_byte_uintvar() {
        let input: [u8; 1] = [0b00000101];
        let res = uintvar(&input);

        let val = res.unwrap().1;
        assert_eq!(val, 5);
    }

    #[test]
    fn read_2_byte_uintvar() {
        let input: [u8; 2] = [0b10000101, 0b00000001];
        assert_eq!(uintvar(&input).unwrap().1, 0b1010000001);
    }

    #[test]
    fn read_multi_byte_uintvar() {
        let input: [u8; 4] = [0b10000001, 0b10000000, 0b10000000, 0b00000011];
        assert_eq!(uintvar(&input).unwrap().1, 0b1000000000000000000011);
    }

    #[test]
    fn take_uintvar_byte_without_carry() {
        let input: [u8; 1] = [0b00000101];
        let (_new_input, (carry, number)) = take_uintvar_byte(&input).unwrap();

        assert_eq!(carry, 0);
        assert_eq!(number, 0b101);
    }

    #[test]
    fn take_uintvar_byte_with_carry() {
        let input: [u8; 1] = [0b10000101];
        let (_new_input, (carry, number)) = take_uintvar_byte(&input).unwrap();

        assert_eq!(carry, 1);
        assert_eq!(number, 0b101);
    }
}
