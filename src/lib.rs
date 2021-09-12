#![allow(unused)]
#[macro_use]
extern crate serde;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate anyhow;

use anyhow::*;
use feishu::FeishuClient;

mod biz;
pub mod config;
pub mod db;
pub mod feishu;
mod http;
