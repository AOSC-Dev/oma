const BASH_COMPLETIONS: &str = include_str!("../../data/completions/oma.bash");
const FISH_COMPLETIONS: &str = include_str!("../../data/completions/oma.fish");

pub fn execute(shell: &str) -> i32 {
    match shell {
        "bash" => println!("{}", BASH_COMPLETIONS),
        "fish" => println!("{}", FISH_COMPLETIONS),
        _ => unreachable!(),
    }

    0
}
