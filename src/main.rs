use argh::FromArgs;
use port_killer::kill;

#[derive(FromArgs)]
#[argh(description = "Please, specify port to kill")]

struct Args {
    #[argh(option, short = 'p', description = "specify port")]
    port: u16,
}

fn main() {
    let args: Args = argh::from_env();

    let port = args.port;

    match kill(port) {
        Ok(result) => println!("{}!", result),
        Err(e) => eprintln!("Something wrong :( \n  {}", e),
    };
}
