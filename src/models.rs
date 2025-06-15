use std::{collections::BTreeMap, sync::Arc};

use gpui::{Hsla, rgb};

#[derive(Debug)]
pub struct DatabaseHeader {
    pub magic: [u8; 16],
    pub page_size: u16,
    pub file_format_write_version: u8,
    pub file_format_read_version: u8,
    pub reserved_space: u8,
    pub max_embedded_payload_fraction: u8,
    pub min_embedded_payload_fraction: u8,
    pub leaf_payload_fraction: u8,
    pub file_change_counter: u32,
    pub database_size_pages: u32,
    pub first_freelist_trunk_page: u32,
    pub total_freelist_pages: u32,
    pub schema_cookie: u32,
    pub schema_format_number: u32,
    pub default_page_cache_size: u32,
    pub largest_root_btree_page: u32,
    pub text_encoding: u32,
    pub user_version: u32,
    pub incremental_vacuum_mode: u32,
    pub application_id: u32,
    pub version_valid_for: u32,
    pub sqlite_version_number: u32,
}

impl DatabaseHeader {
    pub fn actual_page_size(&self) -> usize {
        if self.page_size == 1 {
            65536
        } else {
            self.page_size as usize
        }
    }

    pub fn is_valid_sqlite_file(&self) -> bool {
        &self.magic[..16] == b"SQLite format 3\0"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PageType {
    TableBTreeInterior = 0x05,
    IndexBTreeInterior = 0x02,
    TableBTreeLeaf = 0x0d,
    IndexBTreeLeaf = 0x0a,
    FreelistTrunk = 100,
    FreelistLeaf = 101,
    PayloadOverflow = 102,
    PointerMap = 103,
    LockByte = 104,
    Unknown = 105,
}

impl PageType {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x02 => PageType::IndexBTreeInterior,
            0x05 => PageType::TableBTreeInterior,
            0x0a => PageType::IndexBTreeLeaf,
            0x0d => PageType::TableBTreeLeaf,
            _ => PageType::Unknown,
        }
    }

    pub fn color(&self) -> Hsla {
        match self {
            PageType::TableBTreeInterior => rgb(0x4CAF50).into(), // Green
            PageType::IndexBTreeInterior => rgb(0x2196F3).into(), // Blue
            PageType::TableBTreeLeaf => rgb(0x8BC34A).into(),     // Light Green
            PageType::IndexBTreeLeaf => rgb(0x03DAC6).into(),     // Cyan
            PageType::FreelistTrunk => rgb(0xFF9800).into(),      // Orange
            PageType::FreelistLeaf => rgb(0xFFEB3B).into(),       // Yellow
            PageType::PayloadOverflow => rgb(0x9C27B0).into(),    // Purple
            PageType::PointerMap => rgb(0xE91E63).into(),         // Pink
            PageType::LockByte => rgb(0x607D8B).into(),           // Blue Grey
            PageType::Unknown => rgb(0x9E9E9E).into(),            // Grey
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PageType::TableBTreeInterior => "Table B-Tree Interior",
            PageType::IndexBTreeInterior => "Index B-Tree Interior",
            PageType::TableBTreeLeaf => "Table B-Tree Leaf",
            PageType::IndexBTreeLeaf => "Index B-Tree Leaf",
            PageType::FreelistTrunk => "Freelist Trunk",
            PageType::FreelistLeaf => "Freelist Leaf",
            PageType::PayloadOverflow => "Payload Overflow",
            PageType::PointerMap => "Pointer Map",
            PageType::LockByte => "Lock Byte",
            PageType::Unknown => "Unknown",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            PageType::TableBTreeInterior => "TBI",
            PageType::IndexBTreeInterior => "IBI",
            PageType::TableBTreeLeaf => "TBL",
            PageType::IndexBTreeLeaf => "IBL",
            PageType::FreelistTrunk => "FLT",
            PageType::FreelistLeaf => "FLL",
            PageType::PayloadOverflow => "POF",
            PageType::PointerMap => "PTR",
            PageType::LockByte => "LCK",
            PageType::Unknown => "UNK",
        }
    }

    pub fn has_rightmost_pointer(&self) -> bool {
        matches!(
            self,
            PageType::TableBTreeInterior | PageType::IndexBTreeInterior
        )
    }
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub page_number: u32,
    pub page_type: PageType,
    pub cell_count: u16,
    pub free_space: u16,
    pub fragmented_bytes: u8,
    pub rightmost_pointer: Option<u32>,
}

impl PageInfo {
    pub fn new(
        page_number: u32,
        page_type: PageType,
        cell_count: u16,
        free_space: u16,
        fragmented_bytes: u8,
        rightmost_pointer: Option<u32>,
    ) -> Self {
        Self {
            page_number,
            page_type,
            cell_count,
            free_space,
            fragmented_bytes,
            rightmost_pointer,
        }
    }

    pub fn utilization_percent(&self, page_size: usize) -> f32 {
        let used_space = page_size as u16 - self.free_space;
        (used_space as f32 / page_size as f32) * 100.0
    }
}

#[derive(Debug)]
pub struct DatabaseInfo {
    pub header: DatabaseHeader,
    pub pages: Arc<BTreeMap<u32, PageInfo>>,
    pub total_file_size: u64,
}

impl DatabaseInfo {
    pub fn new(
        header: DatabaseHeader,
        pages: Arc<BTreeMap<u32, PageInfo>>,
        total_file_size: u64,
    ) -> Self {
        Self {
            header,
            pages,
            total_file_size,
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn get_page_info(&self, page_number: u32) -> Option<&PageInfo> {
        self.pages.get(&page_number)
    }
}
