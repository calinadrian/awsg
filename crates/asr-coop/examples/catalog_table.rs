//! Development helper: prints every catalog item, starter gear included, as the tab-separated table
//! `benilla-formats`' `item_display_audit` reads: `id, name, slot, display id, category`.

fn main() {
    for item in asr_coop::economy::Catalog::load().items {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            item.id, item.name, item.slot, item.display_id, item.category
        );
    }
}
