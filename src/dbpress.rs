// use std::path::Path;
// use std::io::{BufRead, BufReader};
// use std::fs;
// use std::fs::File;
//use connectdb::connectdb;
use rusqlite::Connection;
use crate::connectdb;
#[derive(Debug)]
struct Outpt {
    name: String,
}

pub fn dbpress (conn: &Connection) -> (u32, String) {
     let mut errcode: u32 = 0;
     let mut errstring: String = "good database".to_string();
//     let mut bolok = true;
     if let Err(e) = connectdb(&conn) {
         errstring = format!("data base error: {}", e);
         errcode = 1;
//         bolok = false;
     } else {
         // get list of all tables in database
         let strval = "SELECT name FROM sqlite_master WHERE type = \"table\" ";
         match conn.prepare(strval) {
            Ok(mut ss) => {
               match ss.query_map([], |row| {
                Ok(Outpt {
                    name: row.get(0)?,
                })
              }) {
                Ok(ss_iter) => {
                    let mut numtables = 0;
                    let mut tablena: String = "---".to_string();
                    for si in ss_iter {
                         numtables = numtables + 1;
                         let sii = si.unwrap();
                         tablena = sii.name.to_string();
                    }
                    // check to see if blubackup is the only table
                    if numtables == 0 {
                        errstring  = format!("no tables in database: tablena: {}", tablena);
                        errcode = 1;
//                        bolok = false;
                    } else if !(numtables == 1) {  
                        errstring  = format!("{} tables in database: last tablena: {}", numtables, tablena);
                        errcode = 2;
//                        bolok = false;
                    } else {
                        if !(tablena == "blubackup") {
                            errstring  = format!("invalid table of {}", tablena);
                            errcode = 3;
//                            bolok = false;
                        } else {
                            match conn.prepare("SELECT GROUP_CONCAT(NAME,',') FROM PRAGMA_TABLE_INFO('blubackup')") {
                               Ok(mut ssx) => {
                                   match ssx.query_map([], |row| {
                                Ok(Outpt {
                                     name: row.get(0)?,
                                })
                              }) {
                                Ok(ssx_iter) => {
                                    for six in ssx_iter {
                                        let _siix = six.unwrap();
//                                        println!("column listing output {:?}", siix.name);
                                   }
                                }
                                Err(err) => {
                                    errstring  = format!("Error doing sql select group {:?}", err);
                                    errcode = 4;
//                                    bolok = false; 
                                }
                              };
                               }
                               Err(err) => {
                                   errstring  = format!("Error doing sql select group {:?}", err);
                                   errcode = 4;
//                                   bolok = false;
                               } 
                            }        
                         }
                    }                     
                }
                Err(err) => {
                    errstring  = format!("Error doing sql select group {:?}", err);
                    errcode = 4;
//                    bolok = false;

                }
              }
            }
            Err(err) => {
                errstring  = format!("Error doing sql select name {:?}", err);
                errcode = 1;
//                bolok = false;
            } 
         };
     }
     (errcode, errstring)
}

