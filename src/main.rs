#[macro_use]
extern crate honggfuzz;

const ALPHABET: &[u8] = b"ATCG";
const MAX_REPEAT: u32 = 10;

fn fuzz_cycle(data: &[u8]) -> Result<(), ()> {
    use arbitrary::{Arbitrary, FiniteBuffer};
    use grep_regex::RegexMatcher;
    use grep_searcher::sinks::UTF8;
    use grep_searcher::Searcher;
    use rand::SeedableRng;
    use regex_generate::Generator;

    let mut ring = FiniteBuffer::new(&data, data.len()).map_err(|_| ())?;

    let mut regex_bytes: Vec<u8> = Arbitrary::arbitrary(&mut ring)?;
    for byte in &mut regex_bytes {
        *byte = ALPHABET[*byte as usize % ALPHABET.len()];
    }
    let mut regex_string = String::from_utf8(regex_bytes).unwrap();
    let pos: usize = usize::arbitrary(&mut ring)? % (regex_string.len() + 1);
    let (min, max): (Option<u32>, Option<u32>) = Arbitrary::arbitrary(&mut ring)?;
    let min_string = min
        .filter(|n| *n != 0)
        .map_or(String::new(), |n| (n % MAX_REPEAT).to_string());
    let max_string = max
        .filter(|n| *n != 0)
        .map_or(String::new(), |n| (n % MAX_REPEAT).to_string());
    regex_string.insert_str(pos, &format!("[ATCG]{{{},{}}}", min_string, max_string));
    // regex_string = String::from("TTGAGTCCAGGAG[TAGC]{2}C");

    let matcher = RegexMatcher::new_line_matcher(&regex_string).map_err(|_| ())?;
    let seed: u64 = Arbitrary::arbitrary(&mut ring)?;
    let rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
    let mut gen = Generator::new(&regex_string, rng, MAX_REPEAT)
        .expect("regex at this point should be valid");

    let mut searcher = Searcher::new();

    let mut buffer = vec![];
    gen.generate(&mut buffer)
        .expect("should succeed generating text");
    let needle = String::from_utf8(buffer).unwrap();
    // let needle = String::from("TTGAGTCCAGGAGTTC");

    if needle.is_empty() || needle == regex_string {
        return Ok(());
    }

    let mut haystack_bytes: Vec<u8> = Arbitrary::arbitrary(&mut ring)?;
    for byte in &mut haystack_bytes {
        *byte = ALPHABET[*byte as usize % ALPHABET.len()];
    }

    let mut haystack: String = String::from_utf8(haystack_bytes).unwrap();
    let pos: usize = usize::arbitrary(&mut ring)? % (haystack.len() + 1);
    haystack.insert_str(pos, &needle);

    let mut found = false;
    searcher
        .search_slice(
            &matcher,
            haystack.as_bytes(),
            UTF8(|_, _| {
                found = true;
                Ok(true)
            }),
        )
        .unwrap();
    assert!(found);

    Ok(())
}

fn main() {
    better_panic::install();

    loop {
        fuzz!(|data: &[u8]| {
            let _ = fuzz_cycle(data);
        });
    }
}
