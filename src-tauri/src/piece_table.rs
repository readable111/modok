use std::iter::Iterator;
use std::ops::{Bound, Index};
use std::string;

// use Buffer::*;
// use Location::*;
//

// ===== Buffers =====

pub struct OriginalBuffer {
    pub content: String,
    pub line_starts: Vec<usize>,
}

pub struct AddBuffer {
    pub content: String,
    pub line_starts: Vec<usize>,
}

impl AddBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            line_starts: Vec::new(),
        }
    }

    // append text to the add buffer, and keep track of line starts
    pub fn append(&mut self, text: &str) -> (usize, usize){
        let start = self.content.len();
        self.content.push_str(text);
        self.scan_line_starts(start);
        return ( start, text.len() )
    }

    // scan content from offset, appending new line character starts
    fn scan_line_starts(&mut self, from_offset: usize){
        for (i, c) in self.content[from_offset..].chars().enumerate() {
            if c == '\n' || c == '\r' {
                self.line_starts.push(i);
            }
        }
    }
}

// ===== Piece =====

#[derive(Clone, Debug)]
pub enum BufferKind {
    Original,
    Add,
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub buffer_kind: BufferKind,
    pub start: usize,
    pub length: usize,
    pub line_feed_count: usize,
    pub line_starts: Vec<usize>,  // relative to piece start
}

impl Piece {
    pub fn new(buffer_kind: BufferKind, start: usize, length: usize, line_starts: Vec<usize>) -> Self{
        Self {
            buffer_kind: buffer_kind,
            start: start,
            length: length,
            line_feed_count: line_starts.len(),
            line_starts: line_starts,
        }
    }

    pub fn trim_start(&self, amount: usize) -> Self {
        let new_start = self.start + amount;
        let new_length = self.length - amount;

        let new_line_starts: Vec<usize> = self.line_starts
            .iter()
            .filter(|&&pos| pos >= amount)
            .map(|&pos| pos - amount)
            .collect();

        Piece::new(
            self.buffer_kind.clone(),
            new_start,
            new_length,
            new_line_starts,
        )
    }

    pub fn trim_end(&self, new_length: usize) -> Self {
        let new_line_starts: Vec<usize> = self.line_starts
            .iter()
            .filter( |&&pos| pos < new_length)
            .copied()
            .collect();

        Piece::new(
            self.buffer_kind.clone(),
            self.start,
            new_length,
            new_line_starts
            )
    }

    pub fn split_at(&self, local_offset: usize) -> (Self, Self) {
        let split_2_start = self.start + local_offset;
        let split_1_length = local_offset;
        let split_2_length = self.length - local_offset;
        let split_1_line_starts: Vec<usize> = self.line_starts
            .iter()
            .filter(|&&pos| pos < local_offset)
            .copied()
            .collect();

        let split_2_line_starts: Vec<usize> = self.line_starts
            .iter()
            .filter(|&&pos| pos >= local_offset)
            .map(|&pos| pos - local_offset)
            .collect();

        (
            Piece::new(
                self.buffer_kind.clone(),
                self.start,
                split_1_length,
                split_1_line_starts,
                ),
            Piece::new(
                self.buffer_kind.clone(),
                split_2_start,
                split_2_length,
                split_2_line_starts,
                )
            )
    }

    pub fn offset_to_line_col(&self, local_offset: usize) -> (usize, usize) {
        let line = self.line_starts
            .partition_point(|&pos| pos <= local_offset);

        let col = match line {
            0 => local_offset,
            _ => local_offset - self.line_starts[line - 1],
        };

        (line, col)
    }
}

// ===== Node =====

#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Red,
    Black,
}

pub struct Node {
    pub piece: Piece,
    pub left_char_count: usize,
    pub left_line_count: usize,
    pub color: Color,
    pub left: NodePtr,
    pub right: NodePtr,
    pub parent: NodePtr,
}

// A safe nullable pointer to a heap-allocated Node
// Option<Box<Node>> won't work well for a BST with parent pointers
// so we use an index into a node arena instead
pub type NodePtr = Option<usize>;

// ===== Node Arena =====
// Owns all nodes, handed out by index
// Avoids the self-referential pointer problem in Rust trees

pub struct NodeArena {
    nodes: Vec<Node>,
}

impl NodeArena {
    pub fn new() -> Self;
    pub fn alloc(&mut self, node: Node) -> usize;
    pub fn get(&self, idx: usize) -> &Node;
    pub fn get_mut(&mut self, idx: usize) -> &mut Node;
    pub fn free(&mut self, idx: usize);  // mark slot as reusable
}

// ===== PieceTree =====

pub struct PieceTree {
    pub arena: NodeArena,
    pub root: NodePtr,
    pub original_buffer: OriginalBuffer,
    pub add_buffer: AddBuffer,
    pub total_length: usize,
    pub total_lines: usize,
}

impl PieceTree {
    // read text into original buffer and return self
    pub fn new(text: &str) -> Self;

    // insert text into add buffer, and insert a new node to the tree
    pub fn insert(&mut self, offset: usize, text: &str);

    // delete and rebalance tree
    pub fn delete(&mut self, start: usize, length: usize);

    // get line at line number
    pub fn get_line(&self, line_number: usize) -> Option<String>;

    // get all text in the Piece tree
    pub fn get_text(&self) -> String;

    // get slice of text with start and length
    pub fn get_slice(&self, start: usize, length: usize) -> String;

    // given a raw offset, what col/line is the position
    pub fn offset_to_position(&self, offset: usize) -> Option<(usize, usize)>;

    // given a col/line, what is the offset in te buffer?
    pub fn position_to_offset(&self, line: usize, col: usize) -> Option<usize>;

    // get the length of the buffer
    pub fn len(&self) -> usize;

    // get the line count
    pub fn line_count(&self) -> usize;

    // internal tree navigation
    fn find_node_at_offset(&self, offset: usize) -> Option<(usize, usize)>;  // (node_idx, local_offset)
    fn find_node_at_line(&self, line: usize) -> Option<(usize, usize)>;      // (node_idx, local_line)

    // internal tree mutation
    fn insert_node(&mut self, idx: usize);
    fn remove_node(&mut self, idx: usize);
    fn update_metadata(&mut self, idx: usize);  // walk up recomputing left_char/line_count

    // red-black rebalancing
    fn rotate_left(&mut self, idx: usize);
    fn rotate_right(&mut self, idx: usize);
    fn fix_insert(&mut self, idx: usize);
    fn fix_delete(&mut self, idx: usize);
}
