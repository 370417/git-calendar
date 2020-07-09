use chrono::{prelude::*, Duration};
use git2::{Error, Repository, Sort};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Visualize the past year of git history")]
struct Opt {
    /// Author's email address, or "*" for all commits. Defaults to git's user.email.
    #[structopt(short, long)]
    email: Option<String>,
}

fn main() {
    let opt = Opt::from_args();

    let year = Year::from_today();
    let contributions = match tally_contributions(&year, opt.email) {
        Ok(contributions) => contributions,
        Err(e) => panic!("{}", e),
    };

    println!("    {}", format_months(&year));

    let mut rows = [
        String::from("    "),
        String::from("Mon "),
        String::from("    "),
        String::from("Wed "),
        String::from("    "),
        String::from("Fri "),
        String::from("    "),
    ];

    for week in 0..contributions.len() {
        for weekday in 0..7usize {
            let offset = Duration::weeks(week as i64) + Duration::days(weekday as i64);
            let date = year.initial_sunday.checked_add_signed(offset).unwrap();

            rows[weekday].push_str(if date < year.start || date > year.end {
                " "
            } else {
                match contributions[week][weekday] {
                    0 => ".",
                    1 => "1",
                    2 => "2",
                    3 => "3",
                    4 => "4",
                    5 => "5",
                    6 => "6",
                    7 => "7",
                    8 => "8",
                    9 => "9",
                    _ => "X",
                }
            });
        }
    }

    for row in &rows {
        println!("{}", &row);
    }
}

/// Format the 12 months of the year to line up with the weeks at which they begin.
fn format_months(year: &Year) -> String {
    let mut month = year.first_full_month0();

    let mut txt = String::new();
    let mut txt_i = 0;

    let weeks = year.month_starts();
    let mut weeks_i = 0;

    while weeks_i < 12 {
        if txt_i == weeks[weeks_i] {
            txt.push_str(month0_name(month));
            month = (month + 1) % 12;
            weeks_i += 1;
            txt_i += 3;
        } else {
            txt.push_str(" ");
            txt_i += 1;
        }
    }
    txt
}

fn month0_name(month: usize) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][month]
}

/// Year keeps track of the year leading up to a date (`end`).
struct Year {
    start: Date<Utc>,
    end: Date<Utc>,
    /// The Sunday of the first week. `initial_sunday <= start`
    /// Used to calculate which week a date is part of.
    initial_sunday: Date<Utc>,
}

impl Year {
    fn from_today() -> Year {
        let end = Utc::today();
        let start = one_year_ago(end).succ();
        let initial_sunday = first_day_of_week(start);
        Year {
            start,
            end,
            initial_sunday,
        }
    }

    /// What week a given date falls in. Weeks are numbered chronologically
    /// from self.initial_sunday and are 0-indexed.
    fn week(&self, date: Date<Utc>) -> usize {
        let duration = date.signed_duration_since(self.initial_sunday);
        duration.num_weeks() as usize
    }

    /// How many weeks (aligned around Sunday), partial or full, there are in
    /// this year. Should always be 52 or 53. Perhaps it can reach 54?
    fn num_weeks(&self) -> usize {
        let duration = self.end.signed_duration_since(self.initial_sunday);
        duration.num_weeks() as usize + 1
    }

    /// Return an array of what week each month starts on.
    ///
    /// Months are ordered by chronological appearance, not always Jan-Dec.
    /// Eg if self.start is in March, then the months will be ordered April,
    /// May, ..., February, March.
    fn month_starts(&self) -> [usize; 12] {
        let mut start_weeks = [0; 12];
        let mut date = self.end;
        for i in (0..12).rev() {
            // Move to the start of the month
            date = date.with_day(1).unwrap();
            // Record what week it is
            start_weeks[i] = self.week(date);
            // Move to the previous month
            date = date.pred();
        }
        start_weeks
    }

    /// Return the first full month of this year-long period as a 0-indexed number.
    fn first_full_month0(&self) -> usize {
        // The first full month is always the month after self.end's month.
        // This works even when self.start/self.end are on month borders.
        (self.end.month0() as usize + 1) % 12
    }
}

/// Rewind a date by one calendar year.
///
/// ```
/// assert_eq!(one_year_ago(Utc.ymd(2019, 3, 14)), Utc.ymd(2018, 3, 14));
/// assert_eq!(one_year_ago(Utc.ymd(2018, 12, 31)), Utc.ymd(2017, 12, 31));
/// assert_eq!(one_year_ago(Utc.ymd(2020, 2, 29)), Utc.ymd(2019, 2, 28));
/// ```
fn one_year_ago(date: Date<Utc>) -> Date<Utc> {
    let year = date.year();
    date.with_year(year - 1).unwrap_or(Utc.ymd(year - 1, 2, 28))
}

/// Given a date, return the first day of that week (Sunday, not Monday)
fn first_day_of_week(date: Date<Utc>) -> Date<Utc> {
    let days_since_sunday = date.weekday().num_days_from_sunday() as i64;
    date.checked_sub_signed(Duration::days(days_since_sunday))
        .unwrap()
}

fn tally_contributions(year: &Year, email: Option<String>) -> Result<Vec<[u32; 7]>, Error> {
    let repo = Repository::open_from_env()?;
    let config = repo.config()?;

    let user_email = match email {
        Some(email) => email,
        None => match config.get_entry("user.email")?.value() {
            Some(email) => email.to_owned(),
            None => panic!("user.email is invalid utf-8"),
        },
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME)?;

    // Start iterating from HEAD
    revwalk.push_head()?;

    let mut contributions: Vec<[u32; 7]> = vec![Default::default(); year.num_weeks()];

    // Iterate over all the parents of HEAD
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;

        let seconds = commit.time().seconds();
        let date = Utc.timestamp(seconds, 0).date();

        if date < year.start {
            break;
        }

        // What week it is since start_date, zero indexed
        let week = year.week(date);
        // The weekday as a 0-based index from Sunday
        let weekday = date.weekday().num_days_from_sunday() as usize;

        let author = commit.author();
        let commit_email = match author.email() {
            Some(email) => email,
            None => panic!("commit email is invalid utf-8"),
        };
        // Tally relevant contributions
        if user_email == "*" || commit_email == user_email {
            contributions[week][weekday] += 1;
        }
    }

    Ok(contributions)
}
