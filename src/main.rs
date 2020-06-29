use git2::Repository;

fn main() -> Result<(), git2::Error> {
    let repo = Repository::open_from_env()?;
    let config = repo.config()?;

    let user_email = match config.get_entry("user.email")?.value() {
        Some(email) => email.to_owned(),
        None => panic!("user.email is invalid utf-8"),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TIME)?;
    revwalk.push_head()?;

    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        let time = commit.time();
        let author = commit.author();
        let commit_email = match author.email() {
            Some(email) => email,
            None => panic!("commit email is invalid utf-8"),
        };

        println!("time: {}, email: {}", time.seconds(), commit_email);
    }

    Ok(())
}
