use super::*;

level_generator!(Level1, level1, |gen: &mut LevelGenerator<'a>| {
    gen.platform((-500., -30.), 1000.);
    gen.slider_platform((550., -30.), (1050., -30.), 200., 250.);
    gen.checkpoint((200., -30.));
});
