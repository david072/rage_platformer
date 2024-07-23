use super::*;

level_generator!(Level0, level0, |gen: &mut LevelGenerator<'a>| {
    gen.platform((0., -30.), 1000.);
    gen.platform((-300., 0.), 200.);
    gen.spike((100., -30.));
    gen.slider_platform((600., -30.), (1100., 0.), 200.);
    gen.platform((1400., 0.), 400.);
    gen.ending((1500., 25.));
});
