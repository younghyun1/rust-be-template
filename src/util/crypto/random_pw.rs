use rand::{distributions::Uniform, prelude::Distribution, thread_rng};

const CHARSET_LEN: usize = 90;
const RANDOM_PW_CHARSET: [char; CHARSET_LEN] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '-', '=',
    '[', ']', '{', '}', '|', ';', ':', '\'', ',', '.', '<', '>', '/', '?',
];

pub fn generate_random_password() -> String {
    let mut rng = thread_rng();
    let between = Uniform::from(0..CHARSET_LEN);

    (0..16)
        .map(|_| {
            let idx = between.sample(&mut rng);
            RANDOM_PW_CHARSET[idx]
        })
        .collect()
}
