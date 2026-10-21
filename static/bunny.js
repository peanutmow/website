(function () {
    'use strict';

    const ready = (async function () {
        const module = await import('/wasm-bunny/wasm_bunny.js');
        await module.default();
        window.AsciiBunny = {
            render: function (opts) {
                const text = module.bunny_render(
                    opts.cols | 0,
                    opts.rows | 0,
                    opts.angleX || 0,
                    opts.angleY || 0,
                    opts.ramp || ' .-:~,^;*=|>+\\/?&$%#@',
                    opts.invert !== false
                );
                return text ? text.split('\n') : [];
            }
        };
        return window.AsciiBunny;
    })();

    window.bunnyReady = ready;
})();
