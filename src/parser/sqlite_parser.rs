use crate::models::{DatabaseHeader, DatabaseInfo, PageInfo, PageType};
use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

pub async fn parse_database_file(path: &Path) -> Result<Arc<DatabaseInfo>> {
    let mut file = File::open(path)?;

    // Parse header first
    let header = parse_header(&mut file).await?;

    // Validate it's a SQLite file
    if !header.is_valid_sqlite_file() {
        return Err(anyhow::anyhow!("Not a valid SQLite file"));
    }

    let page_size = header.actual_page_size();

    // Get file size to determine number of pages
    let file_metadata = file.metadata()?;
    let file_size = file_metadata.len();
    let total_pages = (file_size as usize) / page_size;

    let mut pages = BTreeMap::new();

    // Parse each page
    for page_num in 1..=total_pages {
        match parse_page(&mut file, page_num as u32, page_size, &header).await {
            Ok(page_info) => {
                let _ = pages.insert(page_num as u32, page_info);
            }
            Err(e) => {
                // Log error but continue parsing other pages
                eprintln!("Warning: Failed to parse page {}: {}", page_num, e);
            }
        }
    }

    Ok(Arc::new(DatabaseInfo::new(
        header,
        Arc::new(pages),
        file_size,
    )))
}

async fn parse_header(file: &mut File) -> Result<DatabaseHeader> {
    file.seek(SeekFrom::Start(0))?;

    // Read SQLite header (first 100 bytes)
    let mut magic = [0u8; 16];
    file.read_exact(&mut magic)?;

    let page_size = file.read_u16::<BigEndian>()?;
    let file_format_write_version = file.read_u8()?;
    let file_format_read_version = file.read_u8()?;
    let reserved_space = file.read_u8()?;
    let max_embedded_payload_fraction = file.read_u8()?;
    let min_embedded_payload_fraction = file.read_u8()?;
    let leaf_payload_fraction = file.read_u8()?;
    let file_change_counter = file.read_u32::<BigEndian>()?;
    let database_size_pages = file.read_u32::<BigEndian>()?;
    let first_freelist_trunk_page = file.read_u32::<BigEndian>()?;
    let total_freelist_pages = file.read_u32::<BigEndian>()?;
    let schema_cookie = file.read_u32::<BigEndian>()?;
    let schema_format_number = file.read_u32::<BigEndian>()?;
    let default_page_cache_size = file.read_u32::<BigEndian>()?;
    let largest_root_btree_page = file.read_u32::<BigEndian>()?;
    let text_encoding = file.read_u32::<BigEndian>()?;
    let user_version = file.read_u32::<BigEndian>()?;
    let incremental_vacuum_mode = file.read_u32::<BigEndian>()?;
    let application_id = file.read_u32::<BigEndian>()?;

    // Skip reserved bytes (20 bytes)
    file.seek(SeekFrom::Current(20))?;

    let version_valid_for = file.read_u32::<BigEndian>()?;
    let sqlite_version_number = file.read_u32::<BigEndian>()?;

    Ok(DatabaseHeader {
        magic,
        page_size,
        file_format_write_version,
        file_format_read_version,
        reserved_space,
        max_embedded_payload_fraction,
        min_embedded_payload_fraction,
        leaf_payload_fraction,
        file_change_counter,
        database_size_pages,
        first_freelist_trunk_page,
        total_freelist_pages,
        schema_cookie,
        schema_format_number,
        default_page_cache_size,
        largest_root_btree_page,
        text_encoding,
        user_version,
        incremental_vacuum_mode,
        application_id,
        version_valid_for,
        sqlite_version_number,
    })
}

async fn parse_page(
    file: &mut File,
    page_number: u32,
    page_size: usize,
    header: &DatabaseHeader,
) -> Result<PageInfo> {
    let page_offset = ((page_number - 1) as u64) * (page_size as u64);
    file.seek(SeekFrom::Start(page_offset))?;

    // Skip database header on page 1
    let header_offset = if page_number == 1 { 100 } else { 0 };
    if header_offset > 0 {
        file.seek(SeekFrom::Current(header_offset))?;
    }

    // Read page header
    let page_type_byte = file.read_u8()?;

    // Determine page type - freelist trunk pages are special
    let page_type = if page_number == header.first_freelist_trunk_page
        && header.first_freelist_trunk_page != 0
    {
        PageType::FreelistTrunk
    } else {
        PageType::from_byte(page_type_byte)
    };

    let _first_freeblock = file.read_u16::<BigEndian>()?;
    let cell_count = file.read_u16::<BigEndian>()?;
    let cell_content_start = file.read_u16::<BigEndian>()?;
    let fragmented_bytes = file.read_u8()?;

    // Read rightmost pointer for interior pages
    let rightmost_pointer = if page_type.has_rightmost_pointer() {
        Some(file.read_u32::<BigEndian>()?)
    } else {
        None
    };

    // Calculate free space
    let page_header_size = if rightmost_pointer.is_some() {
        12usize
    } else {
        8usize
    };
    let total_header_size = page_header_size + header_offset as usize;
    let cell_pointer_array_size = cell_count as usize * 2;
    let used_header_space = total_header_size + cell_pointer_array_size;

    let content_start = if cell_content_start == 0 {
        page_size as u16
    } else {
        cell_content_start
    };

    let free_space = content_start.saturating_sub(used_header_space as u16);

    Ok(PageInfo::new(
        page_number,
        page_type,
        cell_count,
        free_space,
        fragmented_bytes,
        rightmost_pointer,
    ))
}
