use super::*;

level_generator!(Level0, level0, |gen: &mut LevelGenerator<'a>| {
    gen.platform((-500., -30.), 1000.);
    gen.platform((-400., 0.), 200.);
    gen.spike((100., -30.));
    gen.checkpoint((200., -30.));
    gen.checkpoint((300., -30.));
    gen.slider_platform((550., -30.), (950., -30.), 200., 250.);
    gen.platform((1200., 0.), 400.);
    gen.ending((1500., 0.));
    gen.spike_group(-600., -500., -100.);
    gen.spike_group(550., 950., -100.);
    gen.vertical_spike_group(-500., -30., 10.);
});
