import('/wasm-app/wasm_app.js')
    .then(function (module) {
        return module.default().then(function () { module.start(); });
    })
    .catch(function (error) {
        console.error('wasm-app failed to load', error);
    });
