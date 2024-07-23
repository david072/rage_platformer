use super::*;

level_generator!(Level1, level1, |gen: &mut LevelGenerator<'a>| {
    gen.platform((0., -30.), 1000.);
});
