use iced::widget::{button, column, row, text_input, text, Space, checkbox, progress_bar};
use iced::{Alignment, Element, Command, Application, Settings, Color, Size};
use iced::theme::{Theme};
use iced::executor;
use iced::window;
use iced_futures::futures;
use futures::channel::mpsc;
extern crate chrono;
// use std::path::Path;
use std::io::{Write, BufRead, BufReader};
use std::fs::File;
use std::time::Duration as timeDuration;
use std::time::Instant as timeInstant;
use std::thread::sleep;
use chrono::prelude::*;
use chrono::Local;

extern crate walkdir;
use walkdir::WalkDir;
use rusqlite::{Connection, Result};

use eject::{device::{Device, DriveStatus}, discovery::cd_drives};


mod get_winsize;
mod inputpress;
mod execpress;
mod dbpress;
mod dbfile;
mod connectdb;
mod findmd5sum;
use get_winsize::get_winsize;
use inputpress::inputpress;
use execpress::execpress;
use dbpress::dbpress;
use dbfile::dbfile;
use connectdb::connectdb;
use findmd5sum::findmd5sum;

#[derive(Debug)]
struct Bkup {
      rowid: u64,
      refname: String,
      filename: String,
      dirname: String,
      filesize: u64,
      filedate: String,
      md5sum: Option<String>,
      locations: Option<String>,
      notes: Option<String>,
}

pub fn main() -> iced::Result {

     let mut widthxx: f32 = 1350.0;
     let mut heightxx: f32 = 750.0;
     let (errcode, errstring, widtho, heighto) = get_winsize();
     if errcode == 0 {
         widthxx = widtho as f32 - 20.0;
         heightxx = heighto as f32 - 75.0;
         println!("{}", errstring);
     } else {
         println!("**ERROR {} get_winsize: {}", errcode, errstring);
     }

     Bkmd5sum::run(Settings {
        window: window::Settings {
            size: Size::new(widthxx, heightxx),
            ..window::Settings::default()
        },
        ..Settings::default()
     })
}

struct Bkmd5sum {
    offline_bool: bool,
    dbname: String,
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
    dbconn: Connection,
    tx_send: mpsc::UnboundedSender<String>,
    rx_receive: mpsc::UnboundedReceiver<String>,
}

#[derive(Debug, Clone)]
enum Message {
    DBPressed,
    CheckDBPressed,
    Offline(bool),
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
        ( Self { offline_bool: false,dbname: "--".to_string(),bklabel: "--".to_string(), bkpath: "--".to_string(), msg_value: "no message".to_string(),
               targetdir: "--".to_string(), mess_color: Color::from([0.0, 0.0, 0.0]), alt_bool: false, altname: "--".to_string(), 
               targetname: "--".to_string(), do_progress: false, progval: 0.0, tx_send, rx_receive,
               dbconn: Connection::open_in_memory().unwrap(),
          },
          Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("Backup file list with md5sum -- iced")
    }

    fn update(&mut self, message: Message) -> Command<Message>  {
        match message {
            Message::DBPressed => {
               self.mess_color = Color::from([1.0, 0.0, 0.0]);
               if self.offline_bool {
                   self.msg_value = "request database, but offline checked".to_string();
               } else {           
                   let (errcode, errstr, newinput) = dbfile(self.dbname.clone());
                   self.msg_value = errstr.to_string();
                   if errcode == 0 {
                       let conn = Connection::open(newinput.clone()).unwrap();
                       if let Err(e) = connectdb(&conn) {
                           self.msg_value = format!("data base error {}: {}", newinput, e);
                           self.mess_color = Color::from([1.0, 0.0, 0.0]);
                       } else {
                           let (errcoded, errstrd) = dbpress(&conn);
                           self.msg_value = errstrd.to_string();
                           if errcoded == 0 {
                               self.dbconn = conn;
                               self.dbname = newinput.to_string();
                               self.mess_color = Color::from([0.0, 1.0, 0.0]);
                          }
                       } 
                   }
               }
               Command::none()
            }
            Message::BkPressed => {
               let mut n = 0;   
               let mut mountv = "--".to_string();
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
                                        let mountname = vecline[1].to_string();
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
               self.alt_bool = false;
               self.altname = "--".to_string();
               if n < 1 {
                   self.msg_value = "no dvd drives found".to_string();
               } else {
                   if mountv != "--" {
                       let veclabel: Vec<&str> = mountv.split("/").collect();
                       self.bkpath = mountv.clone();
                       self.bklabel = veclabel[veclabel.len() - 1].to_string();
                       if !self.offline_bool {
                           if let Err(e) = connectdb(&self.dbconn) {
                               self.msg_value = format!("data base error {}: {}", self.dbname, e);
                           } else {
                               let (errcoded, errstrd) = dbpress(&self.dbconn);
                               self.msg_value = errstrd.to_string();
                               if errcoded == 0 {
                                   match self.dbconn.prepare("SELECT  rowid, refname, filename, dirname, filesize, filedate, md5sum, locations, notes
                                      FROM blubackup
                                      WHERE refname = :fil
                                      LIMIT 1") {
                                         Ok(mut stmt) => {
                                             match stmt.query_map(&[(":fil", &self.bklabel)], |row| {
                                                 Ok(Bkup {
                                                          rowid: row.get(0)?,
                                                          refname: row.get(1)?,
                                                          filename: row.get(2)?,
                                                          dirname: row.get(3)?,
                                                          filesize: row.get(4)?,
                                                          filedate: row.get(5)?,
                                                          md5sum: row.get(6)?,
                                                          locations: row.get(7)?,
                                                          notes: row.get(8)?,
                                                })
                                              }) {
                                                 Ok(bk_iter) => {
                                                    let mut numentries = 0;
                                                    let mut filenamex = String::new();
                                                    for bk in bk_iter {
                                                         numentries = numentries + 1;
                                                         let bki = bk.unwrap();
                                                         filenamex = bki.filename;
                                                    } // end for
                                                    if numentries > 0 {
                                                        self.msg_value = format!("found label in db {} {}", self.bklabel, filenamex);
                                                        self.mess_color = Color::from([0.0, 1.0, 0.0]);
                                                    } else {
                                                        self.msg_value = format!("no label in db {}", self.bklabel);
                                                        self.alt_bool = true;
                                                        self.altname = format!("???bkup{}", self.bklabel);
                                                    }
                                                 }
                                                 Err(err) => {
                                                    self.msg_value = format!("sql call aa error {}", err);
                                                 }
                                              }
                                         }
                                         Err(err) => {
                                             self.msg_value = format!("sql call bb update error {}", err);
                                         }
                                   }
                               }
                           }
                       }
                   }
               }
               Command::none()
           }
           Message::CheckDBPressed => {
               self.mess_color = Color::from([1.0, 0.0, 0.0]);
               if self.offline_bool {
                   self.msg_value = "request database checked, but offline checked".to_string();
               } else {           
                   if let Err(e) = connectdb(&self.dbconn) {
                       self.msg_value = format!("data base error {}: {}", self.dbname, e);
                   } else {
                       let (errcoded, errstrd) = dbpress(&self.dbconn);
                       self.msg_value = errstrd.to_string();
                       if errcoded == 0 {
                           let labelval: String;
                           if self.alt_bool {
                               labelval = self.altname.clone();
                           } else {
                               labelval = self.bklabel.clone();
                           }
                           match self.dbconn.prepare("SELECT  rowid, refname, filename, dirname, filesize, filedate, md5sum, locations, notes
                                  FROM blubackup
                                  WHERE refname = :fil
                                  LIMIT 1") {
                                     Ok(mut stmt) => {
                                             match stmt.query_map(&[(":fil", &labelval)], |row| {
                                                 Ok(Bkup {
                                                          rowid: row.get(0)?,
                                                          refname: row.get(1)?,
                                                          filename: row.get(2)?,
                                                          dirname: row.get(3)?,
                                                          filesize: row.get(4)?,
                                                          filedate: row.get(5)?,
                                                          md5sum: row.get(6)?,
                                                          locations: row.get(7)?,
                                                          notes: row.get(8)?,
                                                })
                                              }) {
                                                 Ok(bk_iter) => {
                                                    let mut numentries = 0;
                                                    let mut filenamex = String::new();
                                                    for bk in bk_iter {
                                                         numentries = numentries + 1;
                                                         let bki = bk.unwrap();
                                                         filenamex = bki.filename;
                                                    } // end for
                                                    if numentries > 0 {
                                                        self.msg_value = format!("found label in db {} {}", labelval, filenamex);
                                                        self.mess_color = Color::from([0.0, 1.0, 0.0]);
                                                    } else {
                                                        self.msg_value = format!("no label in db {}", labelval);
                                                    }
                                                 }
                                                 Err(err) => {
                                                    self.msg_value = format!("sql call aa error {}", err);
                                                 }
                                              }
                                         }
                                         Err(err) => {
                                             self.msg_value = format!("sql call bb update error {}", err);
                                         }
                                     }
                               }
                           }
                       }
               Command::none()
            }
            Message::AltnameChanged(value) => { self.altname = value; Command::none() }
            Message::Alt(picked) => {self.alt_bool = picked; Command::none()}
            Message::Offline(picked) => {self.offline_bool = picked; Command::none()}
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
               let labelval: String;
               if self.alt_bool {
                   labelval = self.altname.clone();
               } else {
                   labelval = self.bklabel.clone();
               }
               let (errcode, errstr) = execpress(self.bkpath.clone(), self.targetdir.clone(), labelval.clone(), self.targetname.clone());
               self.msg_value = errstr.to_string();
               if errcode == 0 {
                   self.mess_color = Color::from([0.0, 1.0, 0.0]);
                   Command::perform(Execx::execit(self.bkpath.clone(),self.targetdir.clone(), labelval, self.targetname.clone(), self.tx_send.clone()), Message::ExecxFound)

               } else {
                   self.mess_color = Color::from([1.0, 0.0, 0.0]);
                   Command::none()
               }
            }
            Message::ExecxFound(Ok(exx)) => {
               self.msg_value = exx.errval.clone();
               self.mess_color = Color::from([1.0, 0.0, 0.0]);
               let mut n = 0;
               if exx.errcd == 0 {
                   for path in cd_drives() {
                        match Device::open(path.clone()){
                          Ok(drive) => {
                               n = n + 1;
                               match drive.eject() {
                                 Ok(()) => {
                                    self.mess_color = Color::from([0.0, 1.0, 0.0]);
                                 },
                                 Err(e) => {
                                    self.msg_value = format!("error {} in ejecting blu ray", e);
                                 }
                               }
                          }
                          Err(e) => {
                            self.msg_value = format!("error {} in opening blu ray", e);
                          }
                        }
                   }
                   if n < 1 {
                       self.msg_value = "no dvd drives found".to_string();
                   }
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
                    if lenpg1 == 4 {
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
                                        self.msg_value = format!("md5sum progress: {:.3}gb of {:.3}gb {}", (num_flt/1000000000.0), (dem_flt/1000000000.0), progvec[3]);
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
                 text(&self.msg_value).size(20).style(*&self.mess_color),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![checkbox("Offline (no database)", self.offline_bool).on_toggle(Message::Offline).size(20),
                 button("get database").on_press(Message::DBPressed),
                 text("   database:").size(20),text(&self.dbname).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![button("Search for bluray disc").on_press(Message::BkPressed),
                 text("   bluray label:").size(20),
                 text(&self.bklabel).size(20), text("       bluray mount:").size(20), text(&self.bkpath).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![checkbox("Alternative label", self.alt_bool).on_toggle(Message::Alt).size(20),
                 button("check if in db").on_press(Message::CheckDBPressed),
                 text("        Alternative label name: ").size(20),
                 text_input("No input....", &self.altname)
                            .on_input(Message::AltnameChanged).padding(10).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![button("Target directory Button").on_press(Message::TargetdirPressed),
                 text(&self.targetdir).size(20).width(1000)
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("Target file name: ").size(20),
                 text_input(".hdlist", &self.targetname)
                            .on_input(Message::TargetnameChanged).padding(10).size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![Space::with_width(200),
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
            row![text("execute will validate label with database, then get md5sum for each file and update").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),
            row![text("create output listing in target directory").size(20),
            ].align_items(Alignment::Center).spacing(10).padding(10),

        ]
        .padding(5)
        .align_items(Alignment::Start)
        .into()
    }

    fn theme(&self) -> Theme {
       Theme::Dracula
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
//     const BACKSPACE: char = 8u8 as char;
     let mut errstring  = "Completed blu ray listing".to_string();
     let mut errcode: u32 = 0;
     let mut linenum: u64 = 0;
     let mut szaccum: u64 = 0;
     let mut numrows: u64 = 0;
     let mut xrows: u64 = 1000;
     let mut totalsz: u64 = 0;
     let start_time = timeInstant::now();
     let targetfullname: String = format!("{}/{}", targetdir, targetname);
     let lrperpos = targetname.rfind(".").unwrap();
     let csvfullname: String = format!("{}/{}.csvlist", targetdir, &targetname[0..lrperpos]);
     let mut targetfile = File::create(targetfullname).unwrap();
     let mut csvtargetfile = File::create(csvfullname).unwrap();
     println!("bkpath: {}", bkpath);
     for entryx in WalkDir::new(&bkpath).into_iter().filter_map(|e| e.ok()) {
          if let Ok(metadata) = entryx.metadata() {
              if metadata.is_file() {
                  numrows = numrows + 1;
                  if numrows > xrows {
                     let datenn = Local::now();
                     let msgn = format!("Progress|0|100| at {} for {} files", datenn.format("%H:%M:%S"), numrows);
                     tx_send.unbounded_send(msgn).unwrap();
                     xrows = xrows + 1000;
                  }
                  let file_lenx: u64 = metadata.len();
                  totalsz = totalsz + file_lenx;
              }
          }
     }
     if numrows < 1 {
         errstring  = "no files on disk".to_string();
         errcode = 1;
     } else {
        let diffy = start_time.elapsed();
        let minsy: f64 = diffy.as_secs() as f64/60 as f64;
        let dateyy = Local::now();
        let msgx = format!("Progress|{}|{}| elapsed time {:.1} mins at {} for {} files", szaccum, totalsz, minsy, dateyy.format("%H:%M:%S"), numrows);
        tx_send.unbounded_send(msgx).unwrap();
        for entry in WalkDir::new(&bkpath).into_iter().filter_map(|e| e.ok()) {
          if let Ok(metadata) = entry.metadata() {
              if metadata.is_file() {
                  let fullpath = format!("{}",entry.path().display());
                  let fullpathx = format!("{}",entry.path().display());
                  let lrperpos = fullpath.rfind("/").unwrap();
         		  let file_name = fullpath.get((lrperpos+1)..).unwrap();
                  // remove bkpath from file_dir get its length and add to test to verify
                          let lendir = bkpath.len();
         		  let file_dir = fullpath.get(lendir..(lrperpos)).unwrap();
                  let datetime: DateTime<Local> = metadata.modified().unwrap().into();
                  let file_date = format!("{}.000", datetime.format("%Y-%m-%d %T")); 
                  let file_len: u64 = metadata.len();
//                  let dateff = Local::now();
//                   print!("{}\r{} {} {}        ", BACKSPACE, file_len, file_name, dateff.format("%H:%M:%S"));     
                  let md5sumv = findmd5sum(fullpathx);
                  let stroutput = format!("{}|{}|{}|{}|{}|{}",
                                                  file_name,
                                                  file_len,
                                                  file_date,
                                                  file_dir,
                                                  labelname,
                                                  md5sumv);
                  writeln!(&mut targetfile, "{}", stroutput).unwrap();
                  let strcsvoutput = format!("{}|{}|{}|{}|{}|{}",
                                                  labelname,
                                                  file_name,
                                                  file_dir,
                                                  file_len,
                                                  file_date,
                                                  md5sumv);
                  writeln!(&mut csvtargetfile, "{}", strcsvoutput).unwrap();
                  linenum = linenum + 1;
                  szaccum = szaccum + file_len;
                  let diffx = start_time.elapsed();
                  let minsx: f64 = diffx.as_secs() as f64/60 as f64;
                  let datexx = Local::now();
                  let msgx = format!("Progress|{}|{}| elapsed time {:.1} mins at {} {} of {}", szaccum, totalsz, minsx, datexx.format("%H:%M:%S"), linenum, numrows);
                  tx_send.unbounded_send(msgx).unwrap();
              }
          }
        }
     }
     let msgx = format!("Progress|{}|{}| end of md5sum process", numrows, numrows);
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
