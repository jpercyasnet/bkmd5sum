use iced::widget::{button, column, row, text_input, text, horizontal_space, checkbox, progress_bar};
use iced::{Alignment, Element, Command, Application, Settings, Color};
use iced::theme::{self, Theme};
use iced::executor;
use iced::window;
use iced_futures::futures;
use futures::channel::mpsc;
extern crate chrono;
use std::path::Path;
use std::io::{Write, BufRead, BufReader};
use std::fs::File;
use std::time::Duration as timeDuration;
use std::thread::sleep;
use chrono::prelude::*;
extern crate walkdir;
use walkdir::WalkDir;

use eject::{device::{Device, DriveStatus}, discovery::cd_drives};


mod get_winsize;
mod inputpress;
mod execpress;
mod findmd5sum;
use get_winsize::get_winsize;
use inputpress::inputpress;
use execpress::execpress;
use findmd5sum::findmd5sum;

pub fn main() -> iced::Result {

     let mut widthxx: u32 = 1350;
     let mut heightxx: u32 = 750;
     let (errcode, errstring, widtho, heighto) = get_winsize();
     if errcode == 0 {
         widthxx = widtho - 20;
         heightxx = heighto - 75;
         println!("{}", errstring);
     } else {
         println!("**ERROR {} get_winsize: {}", errcode, errstring);
     }

     Bkmd5sum::run(Settings {
        window: window::Settings {
            size: (widthxx, heightxx),
            ..window::Settings::default()
        },
        ..Settings::default()
     })
}

struct Bkmd5sum {
    bklabel: String,
    bkpath: String,
    mess_color: Color,
    msg_value: String,
    altname: String,
    alt_bool: bool,
    targetdir: String,
    targetname: String,
    do_progress: bool,
    progval: f64,
    tx_send: mpsc::UnboundedSender<String>,
    rx_receive: mpsc::UnboundedReceiver<String>,
}

#[derive(Debug, Clone)]
enum Message {
    BkPressed,
    Alt(bool),
    TargetdirPressed,
    AltnameChanged(String),
    TargetnameChanged(String),
    ExecPressed,
    ExecxFound(Result<Execx, Error>),
    ProgressPressed,
    ProgRtn(Result<Progstart, Error>),
}

impl Application for Bkmd5sum {
    type Message = Message;
    type Theme = Theme;
    type Flags = ();
    type Executor = executor::Default;
    fn new(_flags: Self::Flags) -> (Bkmd5sum, iced::Command<Message>) {
        let (tx_send, rx_receive) = mpsc::unbounded();
        ( Self { bklabel: "--".to_string(), bkpath: "--".to_string(), msg_value: "no message".to_string(), targetdir: "--".to_string(),
               mess_color: Color::from([0.0, 0.0, 0.0]), alt_bool: false, altname: "--".to_string(), 
               targetname: "--".to_string(), do_progress: false, progval: 0.0, tx_send, rx_receive,
 
          },
          Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("Backup file list with md5sum -- iced")
    }

    fn update(&mut self, message: Message) -> Command<Message>  {
        match message {
            Message::BkPressed => {
               let mut n = 0;   
               let mut mountv = String::new();
               for path in cd_drives() {
                  // Print the path
//                println!("{:?}", path);
                  let strpath = format!("{:?}", path);
                  // Access the drive
                  match Device::open(path.clone()){
                    Ok(drive) => {
                         n = n + 1;
                         self.msg_value = "able to open".to_string();
                         match drive.status() {
                           Ok(DriveStatus::Empty) =>
                              self.msg_value = "The tray is closed and no disc is inside".to_string(),
                           Ok(DriveStatus::TrayOpen) =>
                              self.msg_value = "The tray is open".to_string(),
                           Ok(DriveStatus::NotReady) =>
                              self.msg_value = "This drive is not ready yet".to_string(),
                           Ok(DriveStatus::Loaded) => {
                              self.msg_value = "There's a disc inside".to_string();
                              let mut line = String::new();
                              let mut linenum = 0;
                              let file = File::open("/proc/mounts").unwrap(); 
                              let mut reader = BufReader::new(file);
                              loop {
                                   match reader.read_line(&mut line) {
                                     Ok(bytes_read) => {
                                        // EOF: save last file address to restart from this address for next run
                                        if bytes_read == 0 {
                                            break;
                                        }
                                        let vecline: Vec<&str> = line.split(" ").collect();
                                        linenum = linenum + 1;
                                        let devname: String = vecline[0].to_string();
                                        let mut mountname = vecline[1].to_string();
                                        if devname.contains("/dev") {
//                                            println!("{} device: {} has mount of {}", linenum, devname, mountname);
                                            let strtrim: String = strpath[1..(strpath.len() -1)].to_string();
                                            if devname.contains(&strtrim) {
//                                                println!("found disc {} {} with mount of {}", strpath, strtrim, mountname);
                                                mountv = mountname;
                                            }
                                        }
                                        line.clear();
                                     }
                                     Err(err) => {
                                        self.msg_value = format!("error in read proc {} ", err);
                                        break;
                                     }
                                   }
                              }
                           },
                           Err(e) =>
                               self.msg_value = format!("error {} in status dvd", e),
                           }
                    }
                    Err(e) => {
                       self.msg_value = format!("error {} in retracting dvd", e);
                    }
                  }
               }
               self.mess_color = Color::from([1.0, 0.0, 0.0]);
               if n < 1 {
                   self.msg_value = "no dvd drives found".to_string();
               } else {
                   self.msg_value = format!("{} dvd drives available", n);
                   let veclabel: Vec<&str> = mountv.split("/").collect();
                   self.bkpath = mountv.clone();
                   self.mess_color = Color::from([1.0, 0.0, 0.0]);
                   self.bklabel = veclabel[(veclabel.len() - 1)].to_string();
               }
               self.alt_bool = false;
               self.altname = "--".to_string();
               Command::none()
           }
            Message::AltnameChanged(value) => { self.altname = value; Command::none() }
            Message::Alt(picked) => {self.alt_bool = picked; Command::none()}
            Message::TargetnameChanged(value) => { self.targetname = value; Command::none() }
            Message::TargetdirPressed => {
//               let mut inputstr: String = self.targetdir.clone();
               let (errcode, errstr, newinput) = inputpress(self.targetdir.clone());
               self.msg_value = errstr.to_string();
               if errcode == 0 {
                   self.targetdir = newinput.to_string();
                   self.mess_color = Color::from([0.0, 1.0, 0.0]);
               } else {
                   self.mess_color = Color::from([1.0, 0.0, 0.0]);
               }
               Command::none()
            }
            Message::ExecPressed => {
            // validate if bluray still mounted and path the same
            // if alternative is valid repalce bklabel with it
               let (errcode, errstr) = execpress(self.bkpath.clone(), self.targetdir.clone(), self.bklabel.clone(), self.targetname.clone());
               self.msg_value = errstr.to_string();
               if errcode == 0 {
                   self.mess_color = Color::from([0.0, 1.0, 0.0]);
                   Command::perform(Execx::execit(self.bklabel.clone(),self.targetdir.clone(), self.altname.clone(), self.targetname.clone(), self.tx_send.clone()), Message::ExecxFound)

               } else {
                   self.mess_color = Color::from([1.0, 0.0, 0.0]);
                   Command::none()
               }
            }
            Message::ExecxFound(Ok(exx)) => {
               self.msg_value = exx.errval.clone();
               if exx.errcd == 0 {
                   self.mess_color = Color::from([0.0, 1.0, 0.0]);
               } else {
                   self.mess_color = Color::from([1.0, 0.0, 0.0]);
               }
               Command::none()
            }
            Message::ExecxFound(Err(_error)) => {
               self.msg_value = "error in copyx copyit routine".to_string();
               self.mess_color = Color::from([1.0, 0.0, 0.0]);
               Command::none()
            }
            Message::ProgressPressed => {
                   self.do_progress = true;
                   Command::perform(Progstart::pstart(), Message::ProgRtn)
            }
            Message::ProgRtn(Ok(_prx)) => {
              if self.do_progress {
                let mut inputval  = " ".to_string();
                let mut bgotmesg = false;
                let mut b100 = false;
                while let Ok(Some(input)) = self.rx_receive.try_next() {
                   inputval = input;
                   bgotmesg = true;
                }
                if bgotmesg {
                    let progvec: Vec<&str> = inputval[0..].split("|").collect();
                    let lenpg1 = progvec.len();
                    if lenpg1 == 3 {
                        let prog1 = progvec[0].to_string();
                        if prog1 == "Progress" {
                            let num_flt: f64 = progvec[1].parse().unwrap_or(-9999.0);
                            if num_flt < 0.0 {
                                println!("progress numeric not numeric: {}", inputval);
                            } else {
                                let dem_flt: f64 = progvec[2].parse().unwrap_or(-9999.0);
                                if dem_flt < 0.0 {
                                    println!("progress numeric not numeric: {}", inputval);
                                } else {
                                    self.progval = 100.0 * (num_flt / dem_flt);
                                    if dem_flt == num_flt {
                                        b100 = true;
                                    } else {
                                        self.msg_value = format!("Convert progress: {:.3}gb of {:.3}gb", (num_flt/1000000000.0), (dem_flt/1000000000.0));
                                        self.mess_color = Color::from([0.0, 0.0, 1.0]);
                                    }
                                }
                            }
                        } else {
                            println!("message not progress: {}", inputval);
                        }
                    } else {
                        println!("message not progress: {}", inputval);
                    }
                } 
                if b100 {
                    Command::none()   
                } else {         
                    Command::perform(Progstart::pstart(), Message::ProgRtn)
                }
              } else {
                Command::none()
              }
            }
            Message::ProgRtn(Err(_error)) => {
                self.msg_value = "error in Progstart::pstart routine".to_string();
                self.mess_color = Color::from([1.0, 0.0, 0.0]);
               Command::none()
            }

        }
    }

    fn view(&self) -> Element<Message> {
        column![
            row![text("Message:").size(20),
                 text(&self.msg_value).size(30).style(*&self.mess_color),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![button("Search for bluray disc").on_press(Message::BkPressed).style(theme::Button::Secondary),
                 text("   bluray label:").size(20),
                 text(&self.bklabel).size(20), text("       bluray mount:").size(20), text(&self.bkpath).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![checkbox("Alternative label", self.alt_bool, Message::Alt,).size(20),
            text("        Alternative label name: ").size(20),
                 text_input("No input....", &self.altname)
                            .on_input(Message::AltnameChanged).padding(10).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![button("Target directory Button").on_press(Message::TargetdirPressed).style(theme::Button::Secondary),
                 text(&self.targetdir).size(20).width(1000)
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("Target file name: ").size(20),
                 text_input(".hdlist", &self.targetname)
                            .on_input(Message::TargetnameChanged).padding(10).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![horizontal_space(200),
                 button("Exec Button").on_press(Message::ExecPressed),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![button("Start Progress Button").on_press(Message::ProgressPressed).height(50),
                 progress_bar(0.0..=100.0,self.progval as f32),
                 text(format!("{:.2}%", &self.progval)).size(30),
            ].align_items(Alignment::Center).spacing(5).padding(10),
            row![text("need to input database").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("get blu ray disk and get path and label and see if label exists in database").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("button to validate label in database").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("execute will validate label with database, then get md5sum for each file and update").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("create output listing in target directory").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("option to do offline with no label validation and output file for future update").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),

        ]
        .padding(5)
        .align_items(Alignment::Start)
        .into()
    }

    fn theme(&self) -> Theme {
       Theme::Dark
/*          Theme::custom(theme::Palette {
                        background: Color::from_rgb8(240, 240, 240),
                        text: Color::BLACK,
                        primary: Color::from_rgb8(230, 230, 230),
                        success: Color::from_rgb(0.0, 1.0, 0.0),
                        danger: Color::from_rgb(1.0, 0.0, 0.0),
                    })
*/               
    }
}

#[derive(Debug, Clone)]
struct Execx {
    errcd: u32,
    errval: String,
}

impl Execx {
//    const TOTAL: u16 = 807;

    async fn execit(bkpath: String, targetdir: String, labelname: String,  targetname: String, tx_send: mpsc::UnboundedSender<String>,) -> Result<Execx, Error> {
     let errstring  = "Complete harddrive listing".to_string();
     let errcode: u32 = 0;
     let mut linenum: u64 = 0;
     let mut szaccum: u64 = 0;
     let mut numrows: u64 = 0;
     let mut totalsz: u64 = 0;
     let targetfullname: String = format!("{}/{}", targetdir, targetname);
     let mut targetfile = File::create(targetfullname).unwrap();
     for entryx in WalkDir::new(&bkpath).into_iter().filter_map(|e| e.ok()) {
          if let Ok(metadata) = entryx.metadata() {
              if metadata.is_file() {
                  numrows = numrows + 1;
                  let file_lenx: u64 = metadata.len();
                  totalsz = totalsz + file_lenx;
              }
          }
     }
     for entry in WalkDir::new(&bkpath).into_iter().filter_map(|e| e.ok()) {
          if let Ok(metadata) = entry.metadata() {
              if metadata.is_file() {
                  let fullpath = format!("{}",entry.path().display());
//                  let (errcod, errstr, md5sumv) = findmd5sum(fullpath.clone());
                  let md5sumv = findmd5sum(fullpath.clone());
                  let lrperpos = fullpath.rfind("/").unwrap();
         		  let file_name = fullpath.get((lrperpos+1)..).unwrap();
                  // remove bkpath from file_dir get its length and add to test to verify
                          let lendir = bkpath.len();
         		  let file_dir = fullpath.get(lendir..(lrperpos)).unwrap();
                  let datetime: DateTime<Local> = metadata.modified().unwrap().into();
                  let file_date = format!("{}.000", datetime.format("%Y-%m-%d %T")); 
                  let file_len: u64 = metadata.len();
                  let stroutput = format!("{}|{}|{}|{}|{}|{}",
                                                  file_name,
                                                  file_len,
                                                  file_date,
                                                  file_dir,
                                                  labelname,
                                                  md5sumv);
                  writeln!(&mut targetfile, "{}", stroutput).unwrap();
                  linenum = linenum + 1;
                  szaccum = szaccum + file_len;
                  let msgx = format!("Progress|{}|{}", szaccum, totalsz);
                  tx_send.unbounded_send(msgx).unwrap();
              }
          }
     }
     let msgx = format!("Progress|{}|{}", numrows, numrows);
     tx_send.unbounded_send(msgx).unwrap();
     Ok(Execx {
            errcd: errcode,
            errval: errstring,
        })
    }
}
#[derive(Debug, Clone)]
pub enum Error {
//    APIError,
//    LanguageError,
}

// loop thru by sleeping for 5 seconds
#[derive(Debug, Clone)]
pub struct Progstart {
//    errcolor: Color,
//    errval: String,
}

impl Progstart {

    pub async fn pstart() -> Result<Progstart, Error> {
//     let errstring  = " ".to_string();
//     let colorx = Color::from([0.0, 0.0, 0.0]);
     sleep(timeDuration::from_secs(5));
     Ok(Progstart {
//            errcolor: colorx,
//            errval: errstring,
        })
    }
}
