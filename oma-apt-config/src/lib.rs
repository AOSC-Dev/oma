use std::collections::HashMap;

pub use oma_apt::config::Config;
use oma_apt::config::ConfigTree;

fn modify_result(
    tree: ConfigTree,
    res: &mut HashMap<String, HashMap<String, String>>,
    root_path: String,
) {
    use std::collections::VecDeque;
    let mut stack = VecDeque::new();
    stack.push_back((tree, root_path));

    let mut first = true;

    while let Some((node, tree_path)) = stack.pop_back() {
        // 跳过要遍历根节点的相邻节点
        if !first && let Some(entry) = node.sibling() {
            stack.push_back((entry, tree_path.clone()));
        }

        let Some(tag) = node.tag() else {
            continue;
        };

        if let Some(entry) = node.child() {
            stack.push_back((entry, format!("{tree_path}::{tag}")));
        }

        if let Some((k, v)) = node.tag().zip(node.value()) {
            res.entry(tree_path).or_default().insert(k, v);
        }

        first = false;
    }
}

pub fn get_tree(config: &Config, key: &str) -> Vec<(String, HashMap<String, String>)> {
    let mut res = HashMap::new();
    let tree = config.tree(key);

    let Some(tree) = tree else {
        return vec![];
    };

    modify_result(
        tree,
        &mut res,
        key.rsplit_once("::")
            .map(|x| x.0.to_string())
            .unwrap_or_else(|| key.to_string()),
    );

    res.into_iter().collect::<Vec<_>>()
}
