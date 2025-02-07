use rand::{distributions::Uniform, prelude::Distribution, thread_rng};

const CHARSET_LEN: usize = 90;
const RANDOM_PW_CHARSET: [u8; CHARSET_LEN] = [
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
    89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
    115, 116, 117, 118, 119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 33, 64, 35, 36,
    37, 94, 38, 42, 40, 41, 95, 43, 45, 61, 91, 93, 123, 125, 124, 59, 58, 39, 44, 46, 60, 62, 47,
    63,
];

pub fn generate_random_password() -> String {
    let mut rng = thread_rng();
    let between = Uniform::from(0..CHARSET_LEN);

    (0..16)
        .map(|_| {
            let idx = between.sample(&mut rng);
            RANDOM_PW_CHARSET[idx] as char
        })
        .collect()
}
