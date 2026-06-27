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
        let (start, length) = self.add_buffer.append(text);

        let line_starts: Vec<usize> = text
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i)
            .collect();

        let piece = Piece::new(
            BufferKind::Add,
            start,
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
        let mut node_idx = self.root.expect("empty tree");

        loop {
            let node = self.arena.get(node_idx);
            
            // total chars to the left of this node in the document
            let node_start = current_offset + node.left_char_count;
            let node_end = node_start + node.piece.length;

            if offset < node_start {
                // target is somewhere in the left subtree
                node_idx = node.left.expect("tried to get left but no node");

            } else if offset >= node_end {
                // target is somewhere in the right subtree
                // shift current_offset forward by everything to the left of and
                // including this node before descending right
                current_offset = node_end;
                node_idx = node.right.expect("tried to get right but no right child");
            } else if offset == node_start || offset == node_end {
                let new_node = Node {
                    piece: piece.clone(),
                    left_char_count: 0,
                    left_line_count: 0,
                    color: Color::Red,
                    left: None,
                    right: None,
                    parent: None,
                };
                let new_idx = self.arena.alloc(new_node);
                self.insert_node(new_idx);
                self.total_length += length;
                self.total_lines += piece.line_feed_count;
                break;
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
                    piece: piece.clone(),
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
    // simple getters — just return the precomputed totals on the tree
    pub fn len(&self) -> usize {
        self.total_length
    }

    pub fn line_count(&self) -> usize {
        self.total_lines
    }

    // ===== Navigation =====

    fn find_node_at_offset(&self, offset: usize) -> Option<(usize, usize)> {
        // walk the tree using left_char_count to navigate,
        // tracking cumulative offset as we descend.
        // returns (node_idx, local_offset) where local_offset is the
        // offset within the piece itself.
        let mut current = self.root?;
        let mut current_offset = 0;

        loop {
            let node = self.arena.get(current);
            let node_start = current_offset + node.left_char_count;
            let node_end = node_start + node.piece.length;
            let left = node.left;
            let right = node.right;

            if offset < node_start {
                match left {
                    None => return None,
                    Some(l) => current = l,
                }
            } else if offset >= node_end {
                match right {
                    None => return None,
                    Some(r) => {
                        current_offset = node_end;
                        current = r;
                    }
                }
            } else {
                // offset falls within this node's piece
                return Some((current, offset - node_start));
            }
        }
    }

    fn find_node_at_line(&self, line: usize) -> Option<(usize, usize)> {
        // same as find_node_at_offset but navigates by line count
        // using left_line_count instead of left_char_count.
        // returns (node_idx, local_line) where local_line is the
        // line number within the piece itself.
        let mut current = self.root?;
        let mut current_line = 0;

        loop {
            let node = self.arena.get(current);
            let node_line_start = current_line + node.left_line_count;
            let node_line_end = node_line_start + node.piece.line_feed_count;
            let left = node.left;
            let right = node.right;

            if line < node_line_start {
                match left {
                    None => return None,
                    Some(l) => current = l,
                }
            } else if line > node_line_end {
                match right {
                    None => return None,
                    Some(r) => {
                        current_line = node_line_end;
                        current = r;
                    }
                }
            } else {
                return Some((current, line - node_line_start));
            }
        }
    }

    // ===== Text Retrieval =====

    pub fn get_text(&self) -> String {
        // in-order traversal of the tree, concatenating each piece's text
        // from its backing buffer
        let mut result = String::with_capacity(self.total_length);
        self.collect_text(self.root, &mut result);
        result
    }

    fn collect_text(&self, node: NodePtr, result: &mut String) {
        // recursive in-order traversal helper for get_text
        let idx = match node {
            None => return,
            Some(i) => i,
        };

        let node = self.arena.get(idx);
        let left = node.left;
        let right = node.right;
        let piece = &node.piece;

        // visit left subtree first
        self.collect_text(left, result);

        // append this node's piece text from the correct buffer
        let text = match piece.buffer_kind {
            BufferKind::Original => &self.original_buffer.content,
            BufferKind::Add => &self.add_buffer.content,
        };
        result.push_str(&text[piece.start..piece.start + piece.length]);

        // visit right subtree
        self.collect_text(right, result);
    }

    pub fn get_slice(&self, start: usize, length: usize) -> String {
        // find the node containing start, then collect characters
        // until we have gathered `length` bytes
        let mut result = String::with_capacity(length);
        let mut remaining = length;
        let mut current_offset = start;

        while remaining > 0 {
            let (node_idx, local_offset) = match self.find_node_at_offset(current_offset) {
                None => break,
                Some(n) => n,
            };

            let piece = &self.arena.get(node_idx).piece;
            let buffer = match piece.buffer_kind {
                BufferKind::Original => &self.original_buffer.content,
                BufferKind::Add => &self.add_buffer.content,
            };

            // take as many bytes as we need from this piece, or the rest of the piece
            let available = piece.length - local_offset;
            let take = available.min(remaining);
            let piece_start = piece.start + local_offset;
            result.push_str(&buffer[piece_start..piece_start + take]);

            remaining -= take;
            current_offset += take;
        }

        result
    }

    pub fn get_line(&self, line_number: usize) -> Option<String> {
        // find the node that contains the start of this line,
        // then collect text until we hit a newline or end of document
        let (node_idx, local_line) = self.find_node_at_line(line_number)?;

        let mut result = String::new();
        let mut current_node = Some(node_idx);
        let mut is_first_piece = true;

        while let Some(idx) = current_node {
            let piece = &self.arena.get(idx).piece;
            let buffer = match piece.buffer_kind {
                BufferKind::Original => &self.original_buffer.content,
                BufferKind::Add => &self.add_buffer.content,
            };

            // for the first piece, start from the line's position within the piece.
            // for subsequent pieces, start from the beginning.
            let piece_start = if is_first_piece {
                // find the byte offset of local_line within this piece
                if local_line == 0 {
                    piece.start
                } else {
                    piece.start + piece.line_starts[local_line - 1] + 1
                }
            } else {
                piece.start
            };

            let piece_text = &buffer[piece_start..piece.start + piece.length];

            // collect text until we hit a newline
            if let Some(newline_pos) = piece_text.find('\n') {
                result.push_str(&piece_text[..newline_pos]);
                return Some(result);
            } else {
                result.push_str(piece_text);
            }

            is_first_piece = false;

            // move to the next node in-order
            current_node = self.next_node(idx);
        }

        Some(result)
    }

    fn next_node(&self, idx: usize) -> NodePtr {
        // in-order successor — either the leftmost node of the right subtree,
        // or the first ancestor where we come from the left
        let node = self.arena.get(idx);

        if let Some(right) = node.right {
            // go right then all the way left
            let mut current = right;
            while let Some(left) = self.arena.get(current).left {
                current = left;
            }
            return Some(current);
        }

        // walk up until we find a parent we came from the left of
        let mut current = idx;
        loop {
            let parent = match self.arena.get(current).parent {
                None => return None,
                Some(p) => p,
            };
            if self.arena.get(parent).left == Some(current) {
                return Some(parent);
            }
            current = parent;
        }
    }

    // ===== Coordinate Conversion =====

    pub fn offset_to_position(&self, offset: usize) -> Option<(usize, usize)> {
        // find the node and local offset, then use the piece's line_starts
        // to binary search for the line and col within the piece.
        // add the cumulative line count of everything to the left of this node.
        let (node_idx, local_offset) = self.find_node_at_offset(offset)?;

        let node = self.arena.get(node_idx);
        let (piece_line, piece_col) = node.piece.offset_to_line_col(local_offset);

        // count lines in all nodes to the left of this one
        let cumulative_lines = self.count_lines_before(node_idx);

        Some((cumulative_lines + piece_line, piece_col))
    }

    pub fn position_to_offset(&self, line: usize, col: usize) -> Option<usize> {
        // find the node containing this line, then compute the byte offset
        // of the start of that line within the piece, and add col.
        let (node_idx, local_line) = self.find_node_at_line(line)?;

        let node = self.arena.get(node_idx);
        let piece = &node.piece;

        // byte offset of the start of local_line within the piece
        let line_start_in_piece = if local_line == 0 {
            0
        } else {
            piece.line_starts[local_line - 1] + 1
        };

        // count chars in all nodes to the left of this node
        let cumulative_chars = self.count_chars_before(node_idx);

        Some(cumulative_chars + line_start_in_piece + col)
    }

    fn count_lines_before(&self, idx: usize) -> usize {
        // sum up line counts of everything that comes before this node
        // in document order — its left subtree plus ancestors where
        // this node is in the right subtree
        let node = self.arena.get(idx);
        let mut count = node.left_line_count;
        let mut current = idx;

        loop {
            let parent = match self.arena.get(current).parent {
                None => break,
                Some(p) => p,
            };
            // if we came from the right, add the parent's left subtree and the parent piece itself
            if self.arena.get(parent).right == Some(current) {
                let parent_node = self.arena.get(parent);
                count += parent_node.left_line_count + parent_node.piece.line_feed_count;
            }
            current = parent;
        }

        count
    }

    fn count_chars_before(&self, idx: usize) -> usize {
        // same as count_lines_before but for character counts
        let node = self.arena.get(idx);
        let mut count = node.left_char_count;
        let mut current = idx;

        loop {
            let parent = match self.arena.get(current).parent {
                None => break,
                Some(p) => p,
            };
            if self.arena.get(parent).right == Some(current) {
                let parent_node = self.arena.get(parent);
                count += parent_node.left_char_count + parent_node.piece.length;
            }
            current = parent;
        }

        count
    }

    // ===== Delete =====

    pub fn delete(&mut self, start: usize, length: usize) {
        // find where the deletion starts and ends in the tree,
        // then trim/remove pieces across that range.
        if length == 0 {
            return;
        }

        let end = start + length;
        let mut current_offset = start;

        while current_offset < end {
            let (node_idx, local_offset) = match self.find_node_at_offset(current_offset) {
                None => break,
                Some(n) => n,
            };

            let piece_length = self.arena.get(node_idx).piece.length;
            let piece_end = current_offset - local_offset + piece_length;
            let delete_end = end.min(piece_end);
            let delete_length = delete_end - current_offset;

            if local_offset == 0 && delete_length == piece_length {
                // case 1 — deletion covers the entire piece, remove the node
                self.total_length -= piece_length;
                self.total_lines -= self.arena.get(node_idx).piece.line_feed_count;
                self.remove_node(node_idx);

            } else if local_offset == 0 {
                // case 2 — deletion trims the start of the piece
                let new_piece = self.arena.get(node_idx).piece.trim_start(delete_length);
                self.total_length -= delete_length;
                self.total_lines -= self.arena.get(node_idx).piece.line_feed_count - new_piece.line_feed_count;
                self.arena.get_mut(node_idx).piece = new_piece;
                self.update_metadata(node_idx);

            } else if delete_end == piece_end {
                // case 3 — deletion trims the end of the piece
                let new_piece = self.arena.get(node_idx).piece.trim_end(local_offset);
                self.total_length -= delete_length;
                self.total_lines -= self.arena.get(node_idx).piece.line_feed_count - new_piece.line_feed_count;
                self.arena.get_mut(node_idx).piece = new_piece;
                self.update_metadata(node_idx);

            } else {
                // case 4 — deletion is in the middle of the piece, split it
                // left half stays, right half is reinserted, middle is dropped
                let original_piece = self.arena.get(node_idx).piece.clone();
                let (left_piece, right_piece) = original_piece.split_at(local_offset);
                let right_piece = right_piece.trim_start(delete_length - local_offset);

                self.total_length -= delete_length;
                self.total_lines -= original_piece.line_feed_count
                    - left_piece.line_feed_count
                    - right_piece.line_feed_count;

                // update existing node with left piece
                self.arena.get_mut(node_idx).piece = left_piece;

                // insert right piece as a new node
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
                self.update_metadata(node_idx);
            }

            current_offset = delete_end;
        }
    }

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
    fn rotate_left(&mut self, idx: usize) {
        // standard left rotation:
        //
        //     idx                right
        //    /   \              /     \
        //   A    right   =>   idx     C
        //       /    \       /   \
        //      B      C     A     B
        //
        // right takes idx's place, idx becomes right's left child

        let right = self.arena.get(idx).right.expect("rotate_left called with no right child");
        let right_left = self.arena.get(right).left;
        let idx_parent = self.arena.get(idx).parent;

        // move right's left child (B) to idx's right
        self.arena.get_mut(idx).right = right_left;
        if let Some(right_left_idx) = right_left {
            self.arena.get_mut(right_left_idx).parent = Some(idx);
        }

        // right takes idx's place in the tree
        self.arena.get_mut(right).parent = idx_parent;
        match idx_parent {
            None => self.root = Some(right),
            Some(parent) => {
                let parent_node = self.arena.get(parent);
                if parent_node.left == Some(idx) {
                    self.arena.get_mut(parent).left = Some(right);
                } else {
                    self.arena.get_mut(parent).right = Some(right);
                }
            }
        }

        // idx becomes right's left child
        self.arena.get_mut(right).left = Some(idx);
        self.arena.get_mut(idx).parent = Some(right);

        // recompute metadata bottom up — idx first since it is now lower in the tree
        // idx's left subtree hasn't changed, only its right subtree (now B instead of right's subtree)
        let idx_left_char = self.arena.get(idx).left_char_count;
        let idx_left_line = self.arena.get(idx).left_line_count;
        let b_char = right_left.map(|i| {
            let n = self.arena.get(i);
            n.left_char_count + n.piece.length
        }).unwrap_or(0);
        let b_line = right_left.map(|i| {
            let n = self.arena.get(i);
            n.left_line_count + n.piece.line_feed_count
        }).unwrap_or(0);

        // right's new left subtree is now all of idx's subtree
        let right_left_char = idx_left_char + self.arena.get(idx).piece.length + b_char;
        let right_left_line = idx_left_line + self.arena.get(idx).piece.line_feed_count + b_line;
        self.arena.get_mut(right).left_char_count = right_left_char;
        self.arena.get_mut(right).left_line_count = right_left_line;
    }

    fn rotate_right(&mut self, idx: usize) {
        // mirror image of rotate_left:
        //
        //       idx             left
        //      /   \           /    \
        //    left   C   =>    A     idx
        //   /    \                 /   \
        //  A      B               B     C

        let left = self.arena.get(idx).left.expect("rotate_right called with no left child");
        let left_right = self.arena.get(left).right;
        let idx_parent = self.arena.get(idx).parent;

        // move left's right child (B) to idx's left
        self.arena.get_mut(idx).left = left_right;
        if let Some(left_right_idx) = left_right {
            self.arena.get_mut(left_right_idx).parent = Some(idx);
        }

        // left takes idx's place in the tree
        self.arena.get_mut(left).parent = idx_parent;
        match idx_parent {
            None => self.root = Some(left),
            Some(parent) => {
                let parent_node = self.arena.get(parent);
                if parent_node.right == Some(idx) {
                    self.arena.get_mut(parent).right = Some(left);
                } else {
                    self.arena.get_mut(parent).left = Some(left);
                }
            }
        }

        // idx becomes left's right child
        self.arena.get_mut(left).right = Some(idx);
        self.arena.get_mut(idx).parent = Some(left);

        // recompute metadata — idx first since it is now lower
        // idx's new left subtree is B (left_right) instead of left's whole subtree
        let b_char = left_right.map(|i| {
            let n = self.arena.get(i);
            n.left_char_count + n.piece.length
        }).unwrap_or(0);
        let b_line = left_right.map(|i| {
            let n = self.arena.get(i);
            n.left_line_count + n.piece.line_feed_count
        }).unwrap_or(0);

        self.arena.get_mut(idx).left_char_count = b_char;
        self.arena.get_mut(idx).left_line_count = b_line;

        // left's new left subtree is unchanged, so left_char_count/left_line_count stay the same
    }

    fn fix_insert(&mut self, mut idx: usize) {
        // after a BST insert, the new node is Red. we walk up fixing violations
        // of the red-black properties:
        //   1. root must be Black
        //   2. no two consecutive Red nodes (red node cannot have a red parent)

        while self.arena.get(idx).color == Color::Red {
            let parent = match self.arena.get(idx).parent {
                None => break, // hit the root
                Some(p) => p,
            };

            if self.arena.get(parent).color == Color::Black {
                break; // no violation, done
            }

            let grandparent = match self.arena.get(parent).parent {
                None => break,
                Some(g) => g,
            };

            let parent_is_left = self.arena.get(grandparent).left == Some(parent);
            let uncle = if parent_is_left {
                self.arena.get(grandparent).right
            } else {
                self.arena.get(grandparent).left
            };

            let uncle_is_red = uncle
                .map(|u| self.arena.get(u).color == Color::Red)
                .unwrap_or(false);

            if uncle_is_red {
                // case 1 — uncle is red: recolor and move up
                self.arena.get_mut(parent).color = Color::Black;
                if let Some(u) = uncle {
                    self.arena.get_mut(u).color = Color::Black;
                }
                self.arena.get_mut(grandparent).color = Color::Red;
                idx = grandparent;

            } else if parent_is_left {
                if self.arena.get(parent).right == Some(idx) {
                    // case 2 — uncle is black, triangle (left-right): rotate parent left
                    self.rotate_left(parent);
                    idx = parent;
                }
                // case 3 — uncle is black, line (left-left): rotate grandparent right
                let parent = self.arena.get(idx).parent.unwrap();
                let grandparent = self.arena.get(parent).parent.unwrap();
                self.arena.get_mut(parent).color = Color::Black;
                self.arena.get_mut(grandparent).color = Color::Red;
                self.rotate_right(grandparent);

            } else {
                if self.arena.get(parent).left == Some(idx) {
                    // case 2 mirror — triangle (right-left): rotate parent right
                    self.rotate_right(parent);
                    idx = parent;
                }
                // case 3 mirror — line (right-right): rotate grandparent left
                let parent = self.arena.get(idx).parent.unwrap();
                let grandparent = self.arena.get(parent).parent.unwrap();
                self.arena.get_mut(parent).color = Color::Black;
                self.arena.get_mut(grandparent).color = Color::Red;
                self.rotate_left(grandparent);
            }
        }

        // root is always black
        if let Some(root) = self.root {
            self.arena.get_mut(root).color = Color::Black;
        }
    }

    fn fix_delete(&mut self, mut idx: usize) {
        // after a BST delete, we may have a "double black" node that needs fixing.
        // we walk up resolving the violation through rotations and recoloring.

        while Some(idx) != self.root {
            let parent = match self.arena.get(idx).parent {
                None => break,
                Some(p) => p,
            };

            let is_left = self.arena.get(parent).left == Some(idx);
            let sibling = if is_left {
                self.arena.get(parent).right
            } else {
                self.arena.get(parent).left
            };

            let sibling_idx = match sibling {
                None => break,
                Some(s) => s,
            };

            let sibling_color = self.arena.get(sibling_idx).color.clone();

            if sibling_color == Color::Red {
                // case 1 — sibling is red: rotate and recolor to get a black sibling
                self.arena.get_mut(sibling_idx).color = Color::Black;
                self.arena.get_mut(parent).color = Color::Red;
                if is_left {
                    self.rotate_left(parent);
                } else {
                    self.rotate_right(parent);
                }

            } else {
                let sibling_node = self.arena.get(sibling_idx);
                let sibling_left_black = sibling_node.left
                    .map(|i| self.arena.get(i).color == Color::Black)
                    .unwrap_or(true);
                let sibling_right_black = sibling_node.right
                    .map(|i| self.arena.get(i).color == Color::Black)
                    .unwrap_or(true);

                if sibling_left_black && sibling_right_black {
                    // case 2 — sibling's children are both black: recolor sibling
                    self.arena.get_mut(sibling_idx).color = Color::Red;
                    if self.arena.get(parent).color == Color::Red {
                        self.arena.get_mut(parent).color = Color::Black;
                        break;
                    }
                    idx = parent;

                } else if is_left {
                    if sibling_right_black {
                        // case 3 — sibling's right child is black: rotate sibling right
                        if let Some(sl) = self.arena.get(sibling_idx).left {
                            self.arena.get_mut(sl).color = Color::Black;
                        }
                        self.arena.get_mut(sibling_idx).color = Color::Red;
                        self.rotate_right(sibling_idx);
                    }
                    // case 4 — sibling's right child is red: rotate parent left
                    let sibling = self.arena.get(parent).right.unwrap();
                    let parent_color = self.arena.get(parent).color.clone();
                    self.arena.get_mut(sibling).color = parent_color;
                    self.arena.get_mut(parent).color = Color::Black;
                    if let Some(sr) = self.arena.get(sibling).right {
                        self.arena.get_mut(sr).color = Color::Black;
                    }
                    self.rotate_left(parent);
                    break;

                } else {
                    if sibling_left_black {
                        // case 3 mirror — sibling's left child is black: rotate sibling left
                        if let Some(sr) = self.arena.get(sibling_idx).right {
                            self.arena.get_mut(sr).color = Color::Black;
                        }
                        self.arena.get_mut(sibling_idx).color = Color::Red;
                        self.rotate_left(sibling_idx);
                    }
                    // case 4 mirror — sibling's left child is red: rotate parent right
                    let sibling = self.arena.get(parent).left.unwrap();
                    let parent_color = self.arena.get(parent).color.clone();
                    self.arena.get_mut(sibling).color = parent_color;
                    self.arena.get_mut(parent).color = Color::Black;
                    if let Some(sl) = self.arena.get(sibling).left {
                        self.arena.get_mut(sl).color = Color::Black;
                    }
                    self.rotate_right(parent);
                    break;
                }
            }
        }

        // root is always black
        if let Some(root) = self.root {
            self.arena.get_mut(root).color = Color::Black;
        }
    }
}
