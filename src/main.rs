#[macro_use]
extern crate honggfuzz;

const ALPHABET: &[u8] = b"ATCG";
const REGEX_ALPHABET: &[u8] = b"ATCG{}[]?*0123456789";
const MAX_REPEAT: u32 = 10;

fn has_valid_braces(s: &str) -> bool {
    let open: &[u8] = b"[{";
    let close: &[u8] = b"]}";
    let mut kind: Option<usize> = None;
    let mut distance: usize = 0;

    for byte in s.as_bytes() {
        if kind.is_some() {
            distance += 1;
        }
        if let Some(open_kind) = open.iter().position(|b| b == byte) {
            if kind.is_some() {
                return false;
            }
            kind = Some(open_kind);
        } else if let Some(close_kind) = close.iter().position(|b| b == byte) {
            if kind == Some(1) && distance > 3 {
                return false;
            }
            distance = 0;
            if kind != Some(close_kind) {
                return false;
            }
            kind = None;
        }
    }

    kind.is_none()
}

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
        *byte = REGEX_ALPHABET[*byte as usize % REGEX_ALPHABET.len()];
    }
    let regex_string = String::from_utf8(regex_bytes).unwrap();
    // regex_string = String::from("TTGAGTCCAGGAG[TAGC]{2}C");
    if !has_valid_braces(&regex_string) {
        return Err(());
    }

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
