use {
    crate::{command, db::MeigenDatabase, Synced},
    anyhow::Result,
    std::{
        io::{stdin, stdout, Write},
        sync::Arc,
        time::Instant,
    },
    tokio::sync::RwLock,
};

pub struct Console<D: MeigenDatabase> {
    db: Synced<D>,
}

impl<D: MeigenDatabase> Console<D> {
    pub fn new(db: D) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
        }
    }

    pub async fn run(mut self) {
        let mut buf = String::new();
        loop {
            tokio::task::block_in_place(|| {
                print!("> ");
                stdout().flush().unwrap();
                stdin().read_line(&mut buf).unwrap();
            });

            let begin = Instant::now();
            if let Some(Ok(text)) = self.on_input(buf.trim()).await {
                println!("{}", text);
            }
            println!("process took {}ms", begin.elapsed().as_millis());

            buf.clear();
        }
    }

    async fn on_input(&mut self, text: &str) -> Option<Result<String>> {
        let mut splitted = text.split(' ');

        if splitted.next()? != "g!meigen" {
            return None;
        }

        let sub_command = splitted.next()?;

        match sub_command {
            "gophersay" => {
                let id = splitted.next()?.parse().ok()?;
                Some(command::gophersay(Arc::clone(&self.db), id).await)
            }

            "search" => {
                let sub = splitted.next()?;

                match sub {
                    "author" => Some(
                        command::search_author(
                            Arc::clone(&self.db),
                            splitted.next()?,
                            splitted.next().map(|x| x.parse()).transpose().ok()?,
                            splitted.next().map(|x| x.parse()).transpose().ok()?,
                        )
                        .await,
                    ),

                    "content" => Some(
                        command::search_content(
                            Arc::clone(&self.db),
                            splitted.next()?,
                            splitted.next().map(|x| x.parse()).transpose().ok()?,
                            splitted.next().map(|x| x.parse()).transpose().ok()?,
                        )
                        .await,
                    ),
                    _ => None,
                }
            }

            // this can be written smarter with "or_patterns"
            "make" | "help" | "id" | "list" | "random" | "status" | "delete" => {
                // TODO: support more command
                unimplemented!(
                    "currently {} handler on console is unimplemented",
                    sub_command
                )
            }

            _ => None,
        }
    }
}
