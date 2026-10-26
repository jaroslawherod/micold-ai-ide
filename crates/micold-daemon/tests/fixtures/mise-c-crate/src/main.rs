//! Calls the C function `build.rs` compiled and linked.

extern "C" {
    fn hello_answer() -> i32;
}

fn answer() -> i32 {
    // SAFETY: `hello_answer` takes nothing and returns an int; hello.c defines it.
    unsafe { hello_answer() }
}

fn main() {
    println!("answer {}", answer());
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_c_function_answers() {
        assert_eq!(super::answer(), 42);
    }
}
