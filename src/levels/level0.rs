use super::*;

level_generator!(Level0, level0, |gen: &mut LevelGenerator<'a>| {
    gen.platform((-500., -30.), 1000.);
    gen.platform((-400., 0.), 200.);
    gen.spike((100., -30.));
    gen.slider_platform((500., -30.), (1000., -30.), 200., 250.);
    gen.platform((1200., 0.), 400.);
    gen.ending((1500., 0.));
});
