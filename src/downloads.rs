use aria2_ws::{Event, Notification};

pub struct Downloads {
    aria2: aria2_ws::Client,
    downloads: Vec<String>,
}

impl Downloads {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let aria2 = aria2_ws::Client::connect("ws://127.0.0.1:6800/jsonrpc", None)
            .await
            .unwrap();

        Ok(Self {
            aria2,
            downloads: Vec::new(),
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // let (active, waiting, stopped) = tokio::try_join!(
        //     self.aria2.tell_active(),
        //     self.aria2.tell_waiting(0, 100),
        //     self.aria2.tell_stopped(0, 100),
        // )?;
        // println!("{:#?}, {:#?}, {:#?}", active, waiting, stopped);

        let mut events = self.aria2.subscribe_notifications();

        loop {
            let ev = events.recv().await?;
            match ev {
                Notification::Aria2 {
                    gid,
                    event: Event::Start,
                } => {
                    println!("");
                    self.downloads.push(gid);
                }
                Notification::Aria2 {
                    gid,
                    event: Event::Complete | Event::Stop | Event::Error | Event::BtComplete,
                } => {
                    let Some(index) = self.downloads.iter().position(|e| *e == gid) else {
                        break;
                    };
                    self.downloads.swap_remove(index);
                    if self.downloads.len() == 0 {
                        println!("");
                    }
                }
                notification => {
                    println!("{:#?}", notification);
                }
            };
        }

        Ok(())
    }
}
