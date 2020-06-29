use git2::Repository;

fn main() {
    let repo = match Repository::open_from_env() {
        Ok(repo) => repo,
        Err(e) => panic!("failed to find or open repository: {}", e),
    };

    let config = match repo.config() {
        Ok(config) => config,
        Err(e) => panic!("failed to find or open config: {}", e),
    };

    let email = match config.get_entry("user.email") {
        Ok(email_entry) => match email_entry.value() {
            Some(email) => email.to_owned(),
            None => panic!("user.email is invalid utf-8"),
        },
        Err(e) => panic!("failed to find user.email in git config: {}", e),
    };

    println!("{}", email);
}
