//! The recipe catalog, compiled into the program.

/// A recipe: its name and its ingredients, each with a quantity and a unit.
pub struct Recipe {
    pub name: &'static str,
    pub ingredients: &'static [Ingredient],
}

/// One ingredient of a recipe: its name, and how much the recipe needs of it.
pub struct Ingredient {
    pub name: &'static str,
    pub qty: u64,
    pub unit: &'static str,
}

/// The recipes, in catalog order.
pub const CATALOG: &[Recipe] = &[
    Recipe {
        name: "Spaghetti al pomodoro",
        ingredients: &[
            Ingredient {
                name: "spaghetti",
                qty: 320,
                unit: "g",
            },
            Ingredient {
                name: "tomato",
                qty: 400,
                unit: "g",
            },
            Ingredient {
                name: "olive oil",
                qty: 2,
                unit: "tbsp",
            },
            Ingredient {
                name: "garlic",
                qty: 2,
                unit: "pcs",
            },
        ],
    },
    Recipe {
        name: "Omelette",
        ingredients: &[
            Ingredient {
                name: "eggs",
                qty: 3,
                unit: "pcs",
            },
            Ingredient {
                name: "milk",
                qty: 50,
                unit: "ml",
            },
            Ingredient {
                name: "salt",
                qty: 1,
                unit: "pinch",
            },
            Ingredient {
                name: "butter",
                qty: 10,
                unit: "g",
            },
        ],
    },
    Recipe {
        name: "Pancakes",
        ingredients: &[
            Ingredient {
                name: "flour",
                qty: 200,
                unit: "g",
            },
            Ingredient {
                name: "milk",
                qty: 250,
                unit: "ml",
            },
            Ingredient {
                name: "eggs",
                qty: 2,
                unit: "pcs",
            },
            Ingredient {
                name: "sugar",
                qty: 30,
                unit: "g",
            },
            Ingredient {
                name: "butter",
                qty: 20,
                unit: "g",
            },
        ],
    },
    Recipe {
        name: "Insalata caprese",
        ingredients: &[
            Ingredient {
                name: "tomato",
                qty: 300,
                unit: "g",
            },
            Ingredient {
                name: "mozzarella",
                qty: 250,
                unit: "g",
            },
            Ingredient {
                name: "basil",
                qty: 10,
                unit: "g",
            },
            Ingredient {
                name: "olive oil",
                qty: 2,
                unit: "tbsp",
            },
        ],
    },
    Recipe {
        name: "Minestrone",
        ingredients: &[
            Ingredient {
                name: "beans",
                qty: 100,
                unit: "g",
            },
            Ingredient {
                name: "carrot",
                qty: 100,
                unit: "g",
            },
            Ingredient {
                name: "celery",
                qty: 50,
                unit: "g",
            },
            Ingredient {
                name: "tomato",
                qty: 200,
                unit: "g",
            },
            Ingredient {
                name: "onion",
                qty: 100,
                unit: "g",
            },
            Ingredient {
                name: "pasta",
                qty: 80,
                unit: "g",
            },
        ],
    },
];

/// The recipe called `name`, or `None`. The match ignores case and surrounding
/// whitespace, and must otherwise be exact.
pub fn find_recipe(name: &str) -> Option<&'static Recipe> {
    let wanted = name.trim().to_lowercase();
    CATALOG
        .iter()
        .find(|recipe| recipe.name.to_lowercase() == wanted)
}

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
                .any(|item| item.name.to_lowercase() == wanted)
        })
        .collect()
}
