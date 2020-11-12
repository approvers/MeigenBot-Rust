#[readonly::make]
#[derive(Debug)]
pub struct CliOptions {
    pub database: Database,
    pub dest: String,
}

#[derive(Debug)]
pub enum Database {
    File,
    Mongo,
}

pub fn parse() -> Option<CliOptions> {
    let mut args = std::env::args().skip(1);

    let db_name = args.next();
    let path = args.next();

    if db_name.is_none() && path.is_none() {
        println!("usage: [DB: \"file\" or \"mongo\"] [DB destination path (for fileDB) or url (for mongoDB)]");
        return None;
    }

    if db_name.is_none() {
        println!("Please set DB. Possible values are file and mongo.");
        return None;
    }

    let db = match db_name.as_ref().unwrap().as_str() {
        "file" => Database::File,
        "mongo" => Database::Mongo,
        _ => {
            println!("Incorrect value has been set to DB, possible values are file and mongo.");
            return None;
        }
    };

    if path.is_none() {
        println!("Please set database destination.");
        return None;
    }

    let result = CliOptions {
        database: db,
        dest: path.unwrap(),
    };

    Some(result)
}
