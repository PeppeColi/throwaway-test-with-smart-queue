//! The recipe catalog, compiled into the program.

/// A recipe: its name and the names of its ingredients.
pub struct Recipe {
    pub name: &'static str,
    pub ingredients: &'static [&'static str],
}

/// The recipes, in catalog order.
pub const CATALOG: &[Recipe] = &[
    Recipe {
        name: "Spaghetti al pomodoro",
        ingredients: &["spaghetti", "tomato", "olive oil", "garlic"],
    },
    Recipe {
        name: "Omelette",
        ingredients: &["eggs", "milk", "salt", "butter"],
    },
    Recipe {
        name: "Pancakes",
        ingredients: &["flour", "milk", "eggs", "sugar", "butter"],
    },
    Recipe {
        name: "Insalata caprese",
        ingredients: &["tomato", "mozzarella", "basil", "olive oil"],
    },
    Recipe {
        name: "Minestrone",
        ingredients: &["beans", "carrot", "celery", "tomato", "onion", "pasta"],
    },
];

/// The recipes that contain `ingredient` as a whole ingredient name, in catalog
/// order. The match ignores case and surrounding whitespace.
pub fn recipes_with_ingredient(ingredient: &str) -> Vec<&'static Recipe> {
    let wanted = ingredient.trim().to_lowercase();
    CATALOG
        .iter()
        .filter(|recipe| {
            recipe
                .ingredients
                .iter()
                .any(|name| name.to_lowercase() == wanted)
        })
        .collect()
}
