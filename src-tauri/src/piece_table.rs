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

// The arena is just a Vec of optional nodes.
// None means the slot is free/reusable, Some means it's occupied.
pub struct NodeArena {
    nodes: Vec<Option<Node>>,
    free_list: Vec<usize>,  // indices of slots that have been freed and can be reused
}

impl NodeArena {
    // Initialize with an empty vec and empty free list
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
        }
    }

    // If there are any free slots, reuse one instead of growing the vec.
    // Otherwise push onto the end and return the new index.
    // Either way, return the index so the caller can store it as a NodePtr.
    pub fn alloc(&mut self, node: Node) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.nodes[idx] = Some(node);
            idx
        } else {
            self.nodes.push(Some(node));
            self.nodes.len() - 1
        }
    }

    // Just index into the vec and unwrap — panics if the slot is free,
    // which would indicate a bug (dangling NodePtr) in the tree logic.
    pub fn get(&self, idx: usize) -> &Node {
        self.nodes[idx].as_ref().expect("attempted to get a freed node")
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut Node {
        self.nodes[idx].as_mut().expect("attempted to get a freed node")
    }

    // Don't actually remove from the vec — just set the slot to None
    // and push the index onto the free list so alloc can reuse it later.
    // This keeps all other indices stable, which is important because
    // NodePtrs stored in other nodes are just indices into this vec.
    pub fn free(&mut self, idx: usize) {
        self.nodes[idx] = None;
        self.free_list.push(idx);
    }
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
    pub fn new(text: &str) -> Self {
        let mut arena = NodeArena::new();
        let mut add_buffer = AddBuffer::new();

        // scan the original buffer for newlines
        let line_starts: Vec<usize> = text
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i)
            .collect();

        let original_buffer = OriginalBuffer {
            content: text.to_string(),
            line_starts: line_starts.clone(),
        };

        // the root piece covers the entire original buffer
        let piece = Piece::new(
            BufferKind::Original,
            0,
            text.len(),
            line_starts,
        );

        let total_lines = piece.line_feed_count + 1;
        let total_length = piece.length;

        let root_node = Node {
            piece,
            left_char_count: 0,
            left_line_count: 0,
            color: Color::Black,  // root is always black
            left: None,
            right: None,
            parent: None,
        };

        let root = Some(arena.alloc(root_node));

        Self {
            arena,
            root,
            original_buffer,
            add_buffer,
            total_length,
            total_lines,
        }
    }

    // insert text into add buffer, and insert a new node to the tree
    pub fn insert(&mut self, offset: usize, text: &str){
        let (start, len) = self.add_buffer.append(text);

        let line_starts: Vec<usize> = text
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i)
            .collect();

        let piece = Piece::new(
            BufferKind::Add,
            offset,
            text.len(),
            line_starts,
        );

        // walk the tree until the char count left is gt offset, but the
        // char count left of the next node is less than offset.
        // if offset is exactly at current_node.piece start.    
        //      insert new node left of current node
        //offeset lands exactly and the end of current_node.piece
        //      insert new node right of current node
        // offset lands in the middle of a piec
        //      splie the existing piece at the lcal offset into left, right
        //      replace the existing nodes piece wiht the left piece
        //      insert a new node for the added text to the right of it
        //      insert another new node for the right peice to the right of that
        //
        // update total_length and total_lines on the tree
        //
        // wal back up tree from the insertion point, updating left_char_count and left_line count
        // on each ancestor node taht has the modified subtree on the left side
        //
        // rebalance the red_black tree via fix_insert which will perform rotations as needed. make
        // sure rotate_left and rotate_right also update left char count and left line count on the
        // affected nodes, since rotations change wich nodes ar in whose left subtrees'piece
        let mut current_offset = 0;
        let mut node_idx = self.root;

        loop {
            let node = self.arena.get(node_idx);
            
            // total chars to the left of this node in the document
            let node_start = current_offset + node.left_char_count;
            let node_end = node_start + node.piece.length;

            if offset < node_start {
                // target is somewhere in the left subtree
                node_idx = node.left;

            } else if offset >= node_end {
                // target is somewhere in the right subtree
                // shift current_offset forward by everything to the left of and
                // including this node before descending right
                current_offset = node_end;
                node_idx = node.right;

            } else {
                // clone what we need before mutating the arena
                let local_offset = offset - node_start;
                let existing_piece = self.arena.get(node_idx).piece.clone();
                let (left_piece, right_piece) = existing_piece.split_at(local_offset);

                // replace the existing node's piece with the left half in place
                // no need to alloc a new node for it
                self.arena.get_mut(node_idx).piece = left_piece;

                // alloc and insert the new text node
                let new_node = Node {
                    piece,
                    left_char_count: 0,
                    left_line_count: 0,
                    color: Color::Red,
                    left: None,
                    right: None,
                    parent: None,
                };
                let new_idx = self.arena.alloc(new_node);
                self.insert_node(new_idx);

                // alloc and insert the right half of the split
                let right_node = Node {
                    piece: right_piece,
                    left_char_count: 0,
                    left_line_count: 0,
                    color: Color::Red,
                    left: None,
                    right: None,
                    parent: None,
                };
                let right_idx = self.arena.alloc(right_node);
                self.insert_node(right_idx);

                // update metadata and totals
                self.update_metadata(node_idx);
                self.total_length += piece.length;
                self.total_lines += piece.line_feed_count;
                break;
            }
        }
        
    }

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
    fn update_metadata(&mut self, idx: usize) {
        // walk up the tree from idx, recomputing left_char_count and
        // left_line_count on every ancestor where the modified node
        // is in their left subtree. stop when we hit the root.
        let mut current = Some(idx);

        while let Some(curr_idx) = current {
            let node = self.arena.get(curr_idx);
            let parent_idx = node.parent;

            if let Some(parent) = parent_idx {
                let parent_node = self.arena.get(parent);

                // only update if we are coming from the left child
                if parent_node.left == Some(curr_idx) {
                    let (char_count, line_count) = self.subtree_counts(curr_idx);
                    let parent_node = self.arena.get_mut(parent);
                    parent_node.left_char_count = char_count;
                    parent_node.left_line_count = line_count;
                }
            }

            current = parent_idx;
        }
    }

    // helper — sum up total chars and lines in a subtree rooted at idx
    fn subtree_counts(&self, idx: usize) -> (usize, usize) {
        let node = self.arena.get(idx);
        let char_count = node.left_char_count + node.piece.length;
        let line_count = node.left_line_count + node.piece.line_feed_count;

        // add right subtree counts if it exists
        if let Some(right_idx) = node.right {
            let right = self.arena.get(right_idx);
            (
                char_count + right.left_char_count + right.piece.length,
                line_count + right.left_line_count + right.piece.line_feed_count,
            )
        } else {
            (char_count, line_count)
        }
    }

    fn insert_node(&mut self, idx: usize) {
        // standard BST insert — find the right position by char offset,
        // then hang the new node there and update metadata up the tree.
        // fix_insert handles red-black rebalancing afterward.

        let new_piece_length = self.arena.get(idx).piece.length;
        let new_piece_lines = self.arena.get(idx).piece.line_feed_count;

        if self.root.is_none() {
            self.root = Some(idx);
            self.arena.get_mut(idx).color = Color::Black;
            return;
        }

        let mut current = self.root;
        let mut current_offset = 0;

        loop {
            let curr_idx = current.unwrap();
            let node = self.arena.get(curr_idx);
            let node_start = current_offset + node.left_char_count;

            if idx < curr_idx {
                // go left
                if node.left.is_none() {
                    self.arena.get_mut(curr_idx).left = Some(idx);
                    self.arena.get_mut(idx).parent = Some(curr_idx);

                    // update this node's left metadata immediately
                    self.arena.get_mut(curr_idx).left_char_count = new_piece_length;
                    self.arena.get_mut(curr_idx).left_line_count = new_piece_lines;

                    self.update_metadata(curr_idx);
                    self.fix_insert(idx);
                    return;
                }
                current = node.left;
            } else {
                // go right
                if node.right.is_none() {
                    self.arena.get_mut(curr_idx).right = Some(idx);
                    self.arena.get_mut(idx).parent = Some(curr_idx);

                    self.update_metadata(curr_idx);
                    self.fix_insert(idx);
                    return;
                }
                current_offset = node_start + node.piece.length;
                current = node.right;
            }
        }
    }

    fn remove_node(&mut self, idx: usize) {
        // standard BST removal — three cases based on how many children the node has.
        // update metadata up the tree after structural change,
        // then fix_delete handles red-black rebalancing.

        let node = self.arena.get(idx);
        let left = node.left;
        let right = node.right;
        let parent = node.parent;

        match (left, right) {
            // case 1 — leaf node, just detach it
            (None, None) => {
                if let Some(parent_idx) = parent {
                    let parent_node = self.arena.get_mut(parent_idx);
                    if parent_node.left == Some(idx) {
                        parent_node.left = None;
                    } else {
                        parent_node.right = None;
                    }
                    self.update_metadata(parent_idx);
                } else {
                    self.root = None;
                }
            }

            // case 2 — one child, replace node with its child
            (Some(child), None) | (None, Some(child)) => {
                if let Some(parent_idx) = parent {
                    let parent_node = self.arena.get_mut(parent_idx);
                    if parent_node.left == Some(idx) {
                        parent_node.left = Some(child);
                    } else {
                        parent_node.right = Some(child);
                    }
                    self.arena.get_mut(child).parent = Some(parent_idx);
                    self.update_metadata(parent_idx);
                } else {
                    self.root = Some(child);
                    self.arena.get_mut(child).parent = None;
                }
            }

            // case 3 — two children, replace with in-order successor
            (Some(_), Some(right_idx)) => {
                // find the leftmost node in the right subtree
                let mut successor = right_idx;
                while let Some(left_idx) = self.arena.get(successor).left {
                    successor = left_idx;
                }

                // copy successor's piece into this node
                let successor_piece = self.arena.get(successor).piece.clone();
                self.arena.get_mut(idx).piece = successor_piece;

                // now remove the successor (it has at most one child)
                self.remove_node(successor);
                self.update_metadata(idx);
                return;
            }
        }

        self.fix_delete(idx);
        self.arena.free(idx);
    }

    // red-black rebalancing
    fn rotate_left(&mut self, idx: usize);
    fn rotate_right(&mut self, idx: usize);
    fn fix_insert(&mut self, idx: usize);
    fn fix_delete(&mut self, idx: usize);
}
