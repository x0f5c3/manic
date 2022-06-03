use mnemonicode::gen_random_name;

fn main() {
    (0..4).into_iter().for_each(|x| {
        let gened = gen_random_name();
        let pin = gened.as_str()[0..4].to_string();
        println!("Name {}: {}\nRoom {}: {}", x, gened, x, pin);
    })
}
