use object::{Object, ObjectSection};
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    fs,
    path::Path,
};
use typed_arena::Arena;
type RelocationMap = HashMap<usize, object::Relocation>;

trait Reader: gimli::Reader<Offset = usize> + Send + Sync {}

impl<'input, Endian> Reader for gimli::EndianSlice<'input, Endian> where
    Endian: gimli::Endianity + Send + Sync
{
}

pub fn get_vtables_from_file(path: &Path) -> HashMap<String, Vec<VTableElement>> {
    let file = fs::File::open(&path).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
    let object = object::File::parse(&*mmap).unwrap();
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    dump_file(&object, endian).unwrap()
}

#[derive(Debug, Clone)]
struct Relocate<'a, R: gimli::Reader<Offset = usize>> {
    relocations: &'a RelocationMap,
    section: R,
    reader: R,
}

impl<'a, R: gimli::Reader<Offset = usize>> Relocate<'a, R> {
    fn relocate(&self, offset: usize, value: u64) -> u64 {
        if let Some(relocation) = self.relocations.get(&offset) {
            match relocation.kind() {
                object::RelocationKind::Absolute => {
                    if relocation.has_implicit_addend() {
                        // Use the explicit addend too, because it may have the symbol value.
                        return value.wrapping_add(relocation.addend() as u64);
                    } else {
                        return relocation.addend() as u64;
                    }
                }
                _ => {}
            }
        };
        value
    }
}

impl<'a, R: gimli::Reader<Offset = usize>> gimli::Reader for Relocate<'a, R> {
    type Endian = R::Endian;
    type Offset = R::Offset;

    fn read_address(&mut self, address_size: u8) -> gimli::Result<u64> {
        let offset = self.reader.offset_from(&self.section);
        let value = self.reader.read_address(address_size)?;
        Ok(self.relocate(offset, value))
    }

    fn read_length(&mut self, format: gimli::Format) -> gimli::Result<usize> {
        let offset = self.reader.offset_from(&self.section);
        let value = self.reader.read_length(format)?;
        <usize as gimli::ReaderOffset>::from_u64(self.relocate(offset, value as u64))
    }

    fn read_offset(&mut self, format: gimli::Format) -> gimli::Result<usize> {
        let offset = self.reader.offset_from(&self.section);
        let value = self.reader.read_offset(format)?;
        <usize as gimli::ReaderOffset>::from_u64(self.relocate(offset, value as u64))
    }

    fn read_sized_offset(&mut self, size: u8) -> gimli::Result<usize> {
        let offset = self.reader.offset_from(&self.section);
        let value = self.reader.read_sized_offset(size)?;
        <usize as gimli::ReaderOffset>::from_u64(self.relocate(offset, value as u64))
    }

    #[inline]
    fn split(&mut self, len: Self::Offset) -> gimli::Result<Self> {
        let mut other = self.clone();
        other.reader.truncate(len)?;
        self.reader.skip(len)?;
        Ok(other)
    }

    // All remaining methods simply delegate to `self.reader`.

    #[inline]
    fn endian(&self) -> Self::Endian {
        self.reader.endian()
    }

    #[inline]
    fn len(&self) -> Self::Offset {
        self.reader.len()
    }

    #[inline]
    fn empty(&mut self) {
        self.reader.empty()
    }

    #[inline]
    fn truncate(&mut self, len: Self::Offset) -> gimli::Result<()> {
        self.reader.truncate(len)
    }

    #[inline]
    fn offset_from(&self, base: &Self) -> Self::Offset {
        self.reader.offset_from(&base.reader)
    }

    #[inline]
    fn offset_id(&self) -> gimli::ReaderOffsetId {
        self.reader.offset_id()
    }

    #[inline]
    fn lookup_offset_id(&self, id: gimli::ReaderOffsetId) -> Option<Self::Offset> {
        self.reader.lookup_offset_id(id)
    }

    #[inline]
    fn find(&self, byte: u8) -> gimli::Result<Self::Offset> {
        self.reader.find(byte)
    }

    #[inline]
    fn skip(&mut self, len: Self::Offset) -> gimli::Result<()> {
        self.reader.skip(len)
    }

    #[inline]
    fn to_slice(&self) -> gimli::Result<Cow<[u8]>> {
        self.reader.to_slice()
    }

    #[inline]
    fn to_string(&self) -> gimli::Result<Cow<str>> {
        self.reader.to_string()
    }

    #[inline]
    fn to_string_lossy(&self) -> gimli::Result<Cow<str>> {
        self.reader.to_string_lossy()
    }

    #[inline]
    fn read_slice(&mut self, buf: &mut [u8]) -> gimli::Result<()> {
        self.reader.read_slice(buf)
    }
}

impl<'a, R: Reader> Reader for Relocate<'a, R> {}

fn add_relocations(
    relocations: &mut RelocationMap,
    file: &object::File,
    section: &object::Section,
) {
    for (offset64, mut relocation) in section.relocations() {
        let offset = offset64 as usize;
        if offset as u64 != offset64 {
            continue;
        }
        let offset = offset as usize;
        match relocation.kind() {
            object::RelocationKind::Absolute => {
                match relocation.target() {
                    object::RelocationTarget::Symbol(symbol_idx) => {
                        match file.symbol_by_index(symbol_idx) {
                            Ok(symbol) => {
                                let addend =
                                    symbol.address().wrapping_add(relocation.addend() as u64);
                                relocation.set_addend(addend as i64);
                            }
                            Err(_) => {
                                eprintln!(
                                    "Relocation with invalid symbol for section {} at offset 0x{:08x}",
                                    section.name().unwrap(),
                                    offset
                                );
                            }
                        }
                    }
                    object::RelocationTarget::Section(_section_idx) => {}
                }
                if relocations.insert(offset, relocation).is_some() {
                    eprintln!(
                        "Multiple relocations for section {} at offset 0x{:08x}",
                        section.name().unwrap(),
                        offset
                    );
                }
            }
            _ => {
                println!(
                    "Unsupported relocation for section {} at offset 0x{:08x}",
                    section.name().unwrap(),
                    offset
                );
            }
        }
    }
}

#[derive(Debug)]
pub struct VTableElement {
    pub default: String,
    pub name: String,
    pub pos: u64,
}

fn get_structure_vtable<'abbrev, 'unit, 'tree, R: gimli::Reader>(
    node: gimli::EntriesTreeNode<'abbrev, 'unit, 'tree, R>,
    unit: &gimli::Unit<R>,
    dwarf: &gimli::Dwarf<R>,
) -> Result<Vec<VTableElement>, gimli::Error> {
    let mut vtable = Vec::new();
    let mut children = node.children();
    while let Some(node) = children.next()? {
        let entry = node.entry();
        if entry.tag() == gimli::DW_TAG_subprogram {
            if let (Some(name_val), Some(default_val), Some(pos_expr)) = (
                entry.attr_value(gimli::DW_AT_name)?,
                entry.attr_value(gimli::DW_AT_linkage_name)?,
                entry
                    .attr_value(gimli::DW_AT_vtable_elem_location)?
                    .and_then(|x| x.exprloc_value()),
            ) {
                let name_bytes = dwarf.attr_string(&unit, name_val)?;
                let name = gimli::Reader::to_string(&name_bytes)?.to_string();
                let default_bytes = dwarf.attr_string(&unit, default_val)?;
                let default = gimli::Reader::to_string(&default_bytes)?.to_string();
                let mut pos_eval = pos_expr.evaluation(unit.encoding());
                let pos_res = pos_eval.evaluate()?;
                if !matches!(pos_res, gimli::EvaluationResult::Complete) {
                    unimplemented!("{:?}", pos_res);
                }
                let pos_pieces = pos_eval.result();
                let pos_piece = &pos_pieces[0];
                match pos_piece.location {
                    gimli::Location::Address { address: pos } => {
                        vtable.push(VTableElement { name, default, pos })
                    }
                    _ => unimplemented!("{:?}", pos_piece),
                }
            }
        }
    }
    Ok(vtable)
}

fn walk_node<'abbrev, 'unit, 'tree, R: gimli::Reader>(
    node: gimli::EntriesTreeNode<'abbrev, 'unit, 'tree, R>,
    unit: &gimli::Unit<R>,
    dwarf: &gimli::Dwarf<R>,
    vtables: &mut HashMap<String, Vec<VTableElement>>,
) -> Result<(), gimli::Error> {
    let entry = node.entry();

    if entry.tag() == gimli::DW_TAG_structure_type {
        let name_val = entry
            .attr_value(gimli::DW_AT_name)?
            .expect("missing name, this should an Err instead of panicking but I'm lazy");
        let name_bytes = dwarf.attr_string(&unit, name_val)?;
        let name = gimli::Reader::to_string(&name_bytes)?.to_string();

        let vtable = get_structure_vtable(node, unit, dwarf)?;

        vtables.insert(name, vtable);
    } else {
        let mut children = node.children();
        while let Some(node) = children.next()? {
            walk_node(node, unit, dwarf, vtables)?;
        }
    }

    Ok(())
}

fn dump_file(
    object: &object::File,
    endian: gimli::RunTimeEndian,
) -> Result<HashMap<String, Vec<VTableElement>>, gimli::Error> {
    let arena = (Arena::new(), Arena::new());

    // Load a section and return as `Cow<[u8]>`.
    let mut load_section = |id: gimli::SectionId| -> Result<_, object::read::Error> {
        let mut relocations = RelocationMap::default();
        let name = id.name();
        let data = match object.section_by_name(&name) {
            Some(ref section) => {
                add_relocations(&mut relocations, object, section);
                section.uncompressed_data()?
            }
            // Use a non-zero capacity so that `ReaderOffsetId`s are unique.
            None => Cow::Owned(Vec::with_capacity(1)),
        };
        let data_ref = (*arena.0.alloc(data)).borrow();
        let reader = gimli::EndianSlice::new(data_ref, endian);
        let section = reader;
        let relocations = (*arena.1.alloc(relocations)).borrow();
        Ok(Relocate {
            relocations,
            section,
            reader,
        })
    };

    let no_relocations = (*arena.1.alloc(RelocationMap::default())).borrow();
    let no_reader = Relocate {
        relocations: no_relocations,
        section: Default::default(),
        reader: Default::default(),
    };

    let mut vtables = HashMap::new();

    let dwarf = gimli::Dwarf::load(&mut load_section, |_| Ok(no_reader.clone())).unwrap();

    // Iterate over the compilation units.
    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        /*println!(
            "Unit at <.debug_info+0x{:x}>",
            &header.offset().0
        );*/
        let unit = dwarf.unit(header)?;

        let mut tree = unit.entries_tree(None)?;
        let root = tree.root()?;
        walk_node(root, &unit, &dwarf, &mut vtables)?;
    }

    Ok(vtables)
}
