//! CLI render helpers for `tak list` and `tak tree`.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tak_core::model::{TaskLabel, WorkspaceSpec};

const LIST_TASK: &str = "\x1b[1;36m";
const LIST_DEP: &str = "\x1b[1;33m";
const LIST_PUNC: &str = "\x1b[2;37m";
const TREE_TITLE: &str = "\x1b[1;35m";
const TREE_DIM: &str = "\x1b[2;37m";
const RESET: &str = "\x1b[0m";

const TREE_MIN_WIDTH: u16 = 70;
const TREE_MIN_HEIGHT: u16 = 10;

mod output;
mod render;
mod tree_walker;

use output::{buffer_to_plain_text, colorize_tree_output};
use tree_walker::TreeWalker;

pub(crate) use render::{render_list, render_tree};
