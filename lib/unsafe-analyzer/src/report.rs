use std::collections::{HashMap, HashSet};

/*
pub fn summary(unsafe_items: &Vec<UnsafeItem>)
        // HashMap to store counts of each UnsafeKind per crate
        let mut crate_counts: HashMap<String, HashMap<UnsafeKind, usize>> = HashMap::new();

        // Count each UnsafeItem by crate and kind
        for item in unsafe_items {
            let crate_name = item.name.split("::").next().unwrap_or("").to_string();
            let kind_counts = crate_counts.entry(crate_name).or_insert_with(HashMap::new);
            *kind_counts.entry(item.kind.clone()).or_insert(0) += 1;
        }

        // Print header
        println!("{:<20} {:<10} {:<10} {:<10} {:<10}", "CrateName", "Functions", "Blocks", "Impls", "Traits");

        // Print counts for each crate
        for (crate_name, counts) in crate_counts {
            println!(
                "{:<20} {:<10} {:<10} {:<10} {:<10}",
                crate_name,
                counts.get(&UnsafeKind::Function).unwrap_or(&0),
                counts.get(&UnsafeKind::Block).unwrap_or(&0),
                counts.get(&UnsafeKind::Impl).unwrap_or(&0),
                counts.get(&UnsafeKind::Trait).unwrap_or(&0)
            );
        }
    }
*/
}
