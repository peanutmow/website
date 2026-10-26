(function () {
    'use strict';

    var POINTS = 2400000;
    var MAX_WIDTH = 2560;
    var MAX_PIXELS = 8300000;
    var SEARCH = new URLSearchParams(location.search);
    var CALM = SEARCH.has('calm') || window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    var startedAt = performance.now();

    var canvas = document.getElementById('cloud');

    var gl = null;
    try {
        gl = canvas.getContext('webgl2', {
            alpha: false,
            antialias: false,
            depth: false,
            stencil: false,
            premultipliedAlpha: false,
            preserveDrawingBuffer: false,
            powerPreference: 'high-performance'
        });
    } catch (error) {
        gl = null;
    }

    if (!gl) {
        document.body.classList.add('no-webgl');
        return;
    }

    var BACKGROUND_VS = [
        '#version 300 es',
        'layout(location = 0) in vec2 aPos;',
        'out vec2 vUv;',
        'void main() {',
        '    vUv = aPos * 0.5 + 0.5;',
        '    gl_Position = vec4(aPos, 0.0, 1.0);',
        '}'
    ].join('\n');

    var BACKGROUND_FS = [
        '#version 300 es',
        'precision highp float;',
        'in vec2 vUv;',
        'out vec4 fragColor;',
        'uniform vec2 uRes;',
        'uniform float uTime;',
        'float hash21(vec2 p) {',
        '    p = fract(p * vec2(123.34, 456.21));',
        '    p += dot(p, p + 45.32);',
        '    return fract(p.x * p.y);',
        '}',
        'float vnoise(vec2 p) {',
        '    vec2 i = floor(p);',
        '    vec2 f = fract(p);',
        '    f = f * f * (3.0 - 2.0 * f);',
        '    float a = hash21(i);',
        '    float b = hash21(i + vec2(1.0, 0.0));',
        '    float c = hash21(i + vec2(0.0, 1.0));',
        '    float d = hash21(i + vec2(1.0, 1.0));',
        '    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);',
        '}',
        'float fbm(vec2 p) {',
        '    float value = 0.0;',
        '    float amp = 0.5;',
        '    for (int i = 0; i < 6; i++) {',
        '        value += amp * vnoise(p);',
        '        p = p * 2.02 + vec2(1.7, 9.2);',
        '        amp *= 0.5;',
        '    }',
        '    return value;',
        '}',
        'void main() {',
        '    vec2 uv = (gl_FragCoord.xy - 0.5 * uRes) / uRes.y;',
        '    float t = uTime;',
        '    vec2 q = vec2(',
        '        fbm(uv * 2.0 + vec2(t * 0.02, 0.0)),',
        '        fbm(uv * 2.0 + vec2(5.2, 1.3) - vec2(t * 0.017, 0.0))',
        '    );',
        '    float slow = fbm(uv * 2.6 + q * 1.8 + vec2(t * 0.01, -t * 0.008));',
        '    float fast = fbm(uv * 6.0 - q * 2.4 + vec2(0.0, t * 0.006));',
        '    vec3 deep = vec3(0.003, 0.005, 0.013);',
        '    vec3 mid = vec3(0.012, 0.038, 0.085);',
        '    vec3 hot = vec3(0.10, 0.26, 0.55);',
        '    vec3 color = mix(deep, mid, smoothstep(0.22, 0.86, slow));',
        '    color += hot * pow(fast, 3.0) * 0.85;',
        '    color += vec3(0.19, 0.09, 0.44) * pow(slow, 4.0) * 0.6;',
        '    color *= smoothstep(1.4, 0.05, length(uv));',
        '    fragColor = vec4(color, 1.0);',
        '}'
    ].join('\n');

    var POINTS_VS = [
        '#version 300 es',
        'precision highp float;',
        'layout(location = 0) in vec4 aSeed;',
        'uniform mat4 uProj;',
        'uniform mat4 uView;',
        'uniform float uTime;',
        'uniform float uSize;',
        'out vec3 vColor;',
        'out float vRand;',
        'out float vFade;',
        'mat2 rot(float a) {',
        '    float s = sin(a);',
        '    float c = cos(a);',
        '    return mat2(c, -s, s, c);',
        '}',
        'void main() {',
        '    vec3 p = aSeed.xyz;',
        '    float r = length(p.xz);',
        '    float spin = uTime * (0.10 + 0.55 / (1.0 + r));',
        '    p.xz = rot(spin) * p.xz;',
        '    p.y += sin(uTime * 0.45 + r * 1.3 + aSeed.w * 6.2831) * (0.15 + r * 0.02);',
        '    vec4 view = uView * vec4(p, 1.0);',
        '    float depth = max(-view.z, 0.1);',
        '    gl_Position = uProj * view;',
        '    gl_PointSize = clamp(uSize * (0.35 + aSeed.w * aSeed.w * 1.8) / depth, 1.0, 160.0);',
        '    float core = smoothstep(9.0, 0.0, r);',
        '    vColor = mix(vec3(0.35, 0.70, 1.00), vec3(0.95, 0.72, 1.00), aSeed.w) * (0.40 + core * 0.85);',
        '    vRand = aSeed.w;',
        '    vFade = clamp(1.35 - depth / 26.0, 0.25, 1.0);',
        '}'
    ].join('\n');

    var POINTS_FS = [
        '#version 300 es',
        'precision highp float;',
        'in vec3 vColor;',
        'in float vRand;',
        'in float vFade;',
        'out vec4 fragColor;',
        'uniform float uTime;',
        'mat2 rot(float a) {',
        '    float s = sin(a);',
        '    float c = cos(a);',
        '    return mat2(c, -s, s, c);',
        '}',
        'void main() {',
        '    vec2 c = gl_PointCoord * 2.0 - 1.0;',
        '    float r2 = dot(c, c);',
        '    if (r2 > 1.0) discard;',
        '    float glow = exp(-r2 * 3.2);',
        '    float acc = 0.0;',
        '    vec2 q = c * 3.0 + vRand * 6.0;',
        '    for (int i = 0; i < 20; i++) {',
        '        q = rot(0.37) * q + vec2(0.41, -0.29);',
        '        acc += sin(q.x * q.y * 9.0 + vRand * 24.0 + uTime * 0.7);',
        '    }',
        '    float shimmer = 0.86 + 0.14 * sin(acc * 0.35);',
        '    float alpha = glow * glow * 0.030 * vFade * shimmer;',
        '    fragColor = vec4(vColor * alpha, alpha);',
        '}'
    ].join('\n');

    var BRIGHT_FS = [
        '#version 300 es',
        'precision highp float;',
        'in vec2 vUv;',
        'out vec4 fragColor;',
        'uniform sampler2D uTex;',
        'uniform vec2 uTexel;',
        'void main() {',
        '    vec3 sum = vec3(0.0);',
        '    for (int y = -2; y <= 2; y++) {',
        '        for (int x = -2; x <= 2; x++) {',
        '            vec3 sampleColor = texture(uTex, vUv + vec2(float(x), float(y)) * uTexel * 1.5).rgb;',
        '            sum += max(sampleColor - 0.16, 0.0);',
        '        }',
        '    }',
        '    fragColor = vec4(sum / 25.0, 1.0);',
        '}'
    ].join('\n');

    var BLUR_FS = [
        '#version 300 es',
        'precision highp float;',
        'in vec2 vUv;',
        'out vec4 fragColor;',
        'uniform sampler2D uTex;',
        'uniform vec2 uDir;',
        'void main() {',
        '    vec3 acc = vec3(0.0);',
        '    float weightSum = 0.0;',
        '    for (int i = -10; i <= 10; i++) {',
        '        float w = exp(-float(i * i) * 0.06);',
        '        acc += texture(uTex, vUv + uDir * float(i)).rgb * w;',
        '        weightSum += w;',
        '    }',
        '    fragColor = vec4(acc / weightSum, 1.0);',
        '}'
    ].join('\n');

    var COMPOSITE_FS = [
        '#version 300 es',
        'precision highp float;',
        'in vec2 vUv;',
        'out vec4 fragColor;',
        'uniform sampler2D uScene;',
        'uniform sampler2D uBloom;',
        'uniform vec2 uRes;',
        'uniform float uTime;',
        'uniform float uJank;',
        'void main() {',
        '    vec2 d = vUv - 0.5;',
        '    float shift = 0.0018 * uJank;',
        '    vec3 color = vec3(',
        '        texture(uScene, vUv + d * shift).r,',
        '        texture(uScene, vUv).g,',
        '        texture(uScene, vUv - d * shift).b',
        '    );',
        '    color += texture(uBloom, vUv).rgb * 1.05;',
        '    color = 1.0 - exp(-color * 1.45);',
        '    color *= smoothstep(1.35, 0.4, length(d) * 1.45);',
        '    color *= 0.965 + 0.035 * sin(vUv.y * uRes.y * 1.6);',
        '    float grain = fract(sin(dot(vUv * uRes + uTime, vec2(12.9898, 78.233))) * 43758.5453);',
        '    color += (grain - 0.5) * 0.035;',
        '    fragColor = vec4(color, 1.0);',
        '}'
    ].join('\n');

    function compileShader(type, source) {
        var shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            throw new Error(gl.getShaderInfoLog(shader) || 'shader failed');
        }
        return shader;
    }

    function buildProgram(vertexSource, fragmentSource) {
        var program = gl.createProgram();
        var vertex = compileShader(gl.VERTEX_SHADER, vertexSource);
        var fragment = compileShader(gl.FRAGMENT_SHADER, fragmentSource);
        gl.attachShader(program, vertex);
        gl.attachShader(program, fragment);
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            throw new Error(gl.getProgramInfoLog(program) || 'link failed');
        }
        return program;
    }

    function withUniforms(vertexSource, fragmentSource, names) {
        var program = buildProgram(vertexSource, fragmentSource);
        var bundle = { program: program };
        for (var i = 0; i < names.length; i++) {
            bundle[names[i]] = gl.getUniformLocation(program, names[i]);
        }
        return bundle;
    }

    var background = withUniforms(BACKGROUND_VS, BACKGROUND_FS, ['uRes', 'uTime']);
    var points = withUniforms(POINTS_VS, POINTS_FS, ['uProj', 'uView', 'uTime', 'uSize']);
    var bright = withUniforms(BACKGROUND_VS, BRIGHT_FS, ['uTex', 'uTexel']);
    var blur = withUniforms(BACKGROUND_VS, BLUR_FS, ['uTex', 'uDir']);
    var composite = withUniforms(BACKGROUND_VS, COMPOSITE_FS, ['uScene', 'uBloom', 'uRes', 'uTime', 'uJank']);

    function gauss() {
        return (Math.random() + Math.random() + Math.random() - 1.5) * 0.9;
    }

    function buildCloud(count) {
        var data = new Float32Array(count * 4);
        for (var i = 0; i < count; i++) {
            var r = Math.pow(Math.random(), 0.55) * 13.5;
            var arm = (i % 2) * Math.PI;
            var theta = arm + r * 0.42 + gauss() * (0.55 - r * 0.012);
            var spread = 0.25 + r * 0.055;
            var offset = i * 4;
            data[offset] = Math.cos(theta) * r + gauss() * spread * 1.8;
            data[offset + 1] = gauss() * (0.45 + r * 0.035) * Math.max(0.25, 1 - r / 26);
            data[offset + 2] = Math.sin(theta) * r + gauss() * spread * 1.8;
            data[offset + 3] = Math.random();
        }
        return data;
    }

    var quadVao = gl.createVertexArray();
    gl.bindVertexArray(quadVao);
    var quadBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, quadBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    var cloudData = buildCloud(POINTS);
    var pointsVao = gl.createVertexArray();
    gl.bindVertexArray(pointsVao);
    var pointsBuffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, pointsBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, cloudData, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 4, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
    cloudData = null;

    function makeTarget(width, height) {
        var texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        var framebuffer = gl.createFramebuffer();
        gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer);
        gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texture, 0);
        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.bindTexture(gl.TEXTURE_2D, null);
        return { texture: texture, framebuffer: framebuffer, width: width, height: height };
    }

    function destroyTarget(target) {
        if (!target) return;
        gl.deleteFramebuffer(target.framebuffer);
        gl.deleteTexture(target.texture);
    }

    var scene = null;
    var bloomA = null;
    var bloomB = null;
    var pointSize = 62;

    function resize() {
        var ratio = Math.min(window.devicePixelRatio || 1, 2) * (CALM ? 1 : 2.4);
        var cssWidth = Math.max(1, window.innerWidth);
        var cssHeight = Math.max(1, window.innerHeight);
        var width = Math.min(Math.round(cssWidth * ratio), MAX_WIDTH);
        var height = Math.round(width * cssHeight / cssWidth);
        if (width * height > MAX_PIXELS) {
            var squeeze = Math.sqrt(MAX_PIXELS / (width * height));
            width = Math.round(width * squeeze);
            height = Math.round(height * squeeze);
        }
        width = Math.max(width, 2);
        height = Math.max(height, 2);

        canvas.width = width;
        canvas.height = height;

        destroyTarget(scene);
        destroyTarget(bloomA);
        destroyTarget(bloomB);
        scene = makeTarget(width, height);
        bloomA = makeTarget(Math.max(1, width >> 2), Math.max(1, height >> 2));
        bloomB = makeTarget(Math.max(1, width >> 2), Math.max(1, height >> 2));

        pointSize = 124 * (height / cssHeight);
        perspective(projection, 55 * Math.PI / 180, width / height, 0.1, 140);
    }

    var projection = new Float32Array(16);
    var view = new Float32Array(16);
    var eye = new Float32Array(3);
    var center = new Float32Array(3);
    var up = new Float32Array([0, 1, 0]);

    function perspective(out, fov, aspect, near, far) {
        var f = 1 / Math.tan(fov / 2);
        out.fill(0);
        out[0] = f / aspect;
        out[5] = f;
        out[10] = (far + near) / (near - far);
        out[11] = -1;
        out[14] = (2 * far * near) / (near - far);
    }

    function lookAt(out, from, to, direction) {
        var zx = from[0] - to[0];
        var zy = from[1] - to[1];
        var zz = from[2] - to[2];
        var length = Math.sqrt(zx * zx + zy * zy + zz * zz) || 1;
        zx /= length;
        zy /= length;
        zz /= length;

        var xx = direction[1] * zz - direction[2] * zy;
        var xy = direction[2] * zx - direction[0] * zz;
        var xz = direction[0] * zy - direction[1] * zx;
        length = Math.sqrt(xx * xx + xy * xy + xz * xz) || 1;
        xx /= length;
        xy /= length;
        xz /= length;

        var yx = zy * xz - zz * xy;
        var yy = zz * xx - zx * xz;
        var yz = zx * xy - zy * xx;

        out[0] = xx; out[1] = yx; out[2] = zx; out[3] = 0;
        out[4] = xy; out[5] = yy; out[6] = zy; out[7] = 0;
        out[8] = xz; out[9] = yz; out[10] = zz; out[11] = 0;
        out[12] = -(xx * from[0] + xy * from[1] + xz * from[2]);
        out[13] = -(yx * from[0] + yy * from[1] + yz * from[2]);
        out[14] = -(zx * from[0] + zy * from[1] + zz * from[2]);
        out[15] = 1;
    }

    var burn = new Float32Array(1 << 18);
    for (var i = 0; i < burn.length; i++) burn[i] = Math.random();
    var burnChecksum = 0;

    function referencePass() {
        var mask = burn.length - 1;
        var sum = burnChecksum;
        for (var windowIndex = 0; windowIndex < 56; windowIndex++) {
            var offset = (Math.random() * burn.length) | 0;
            for (var j = 0; j < 262144; j++) {
                sum += Math.sin(burn[(offset + j) & mask] * 17 + (j % 89));
            }
        }
        burnChecksum = sum % 1000000;
    }

    var junk = null;
    function churn() {
        junk = new Uint8Array(32 * 1024 * 1024);
        junk[frames % junk.length] = frames & 255;
        junk = null;
    }

    var stallRead = new Uint8Array(128 * 128 * 4);
    function syncStall() {
        gl.readPixels(0, 0, 128, 128, gl.RGBA, gl.UNSIGNED_BYTE, stallRead);
    }

    function render(t) {
        var orbit = t * 0.055;
        eye[0] = Math.sin(orbit) * 29;
        eye[1] = 9 + Math.sin(t * 0.13) * 1.8;
        eye[2] = Math.cos(orbit) * 29;
        lookAt(view, eye, center, up);

        gl.bindFramebuffer(gl.FRAMEBUFFER, scene.framebuffer);
        gl.viewport(0, 0, scene.width, scene.height);
        gl.disable(gl.BLEND);
        gl.clearColor(0, 0, 0, 1);
        gl.clear(gl.COLOR_BUFFER_BIT);

        gl.useProgram(background.program);
        gl.uniform1f(background.uTime, t);
        gl.uniform2f(background.uRes, scene.width, scene.height);
        gl.bindVertexArray(quadVao);
        gl.drawArrays(gl.TRIANGLES, 0, 3);

        gl.enable(gl.BLEND);
        gl.blendFunc(gl.ONE, gl.ONE);
        gl.useProgram(points.program);
        gl.uniformMatrix4fv(points.uProj, false, projection);
        gl.uniformMatrix4fv(points.uView, false, view);
        gl.uniform1f(points.uTime, t);
        gl.uniform1f(points.uSize, pointSize);
        gl.bindVertexArray(pointsVao);
        gl.drawArrays(gl.POINTS, 0, POINTS);
        gl.disable(gl.BLEND);

        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomA.framebuffer);
        gl.viewport(0, 0, bloomA.width, bloomA.height);
        gl.useProgram(bright.program);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, scene.texture);
        gl.uniform1i(bright.uTex, 0);
        gl.uniform2f(bright.uTexel, 1 / scene.width, 1 / scene.height);
        gl.bindVertexArray(quadVao);
        gl.drawArrays(gl.TRIANGLES, 0, 3);

        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomB.framebuffer);
        gl.useProgram(blur.program);
        gl.bindTexture(gl.TEXTURE_2D, bloomA.texture);
        gl.uniform1i(blur.uTex, 0);
        gl.uniform2f(blur.uDir, 1 / bloomA.width, 0);
        gl.drawArrays(gl.TRIANGLES, 0, 3);

        gl.bindFramebuffer(gl.FRAMEBUFFER, bloomA.framebuffer);
        gl.bindTexture(gl.TEXTURE_2D, bloomB.texture);
        gl.uniform2f(blur.uDir, 0, 1 / bloomB.height);
        gl.drawArrays(gl.TRIANGLES, 0, 3);

        gl.bindFramebuffer(gl.FRAMEBUFFER, null);
        gl.viewport(0, 0, canvas.width, canvas.height);
        gl.useProgram(composite.program);
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, scene.texture);
        gl.uniform1i(composite.uScene, 0);
        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, bloomA.texture);
        gl.uniform1i(composite.uBloom, 1);
        gl.uniform2f(composite.uRes, canvas.width, canvas.height);
        gl.uniform1f(composite.uTime, t);
        gl.uniform1f(composite.uJank, 0.6 + 0.4 * Math.sin(t * 0.6));
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        gl.activeTexture(gl.TEXTURE0);
    }

    var workers = [];

    function startWorkers(total) {
        var source = [
            'var total = 0;',
            'var running = true;',
            'onmessage = function (event) {',
            '    if (event.data === "pause") running = false;',
            '    if (event.data === "resume") running = true;',
            '};',
            'function spin() {',
            '    if (running) {',
            '        var deadline = performance.now() + 220;',
            '        while (performance.now() < deadline) {',
            '            for (var i = 0; i < 4096; i++) {',
            '                total = (total + Math.sqrt(i + (total % 101)) * 0.5) % 1000000;',
            '            }',
            '        }',
            '        postMessage(total | 0);',
            '        total = 0;',
            '    }',
            '    setTimeout(spin, running ? 0 : 250);',
            '}',
            'spin();'
        ].join('\n');
        var url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
        for (var index = 0; index < total; index++) {
            try {
                var worker = new Worker(url);
                worker.onmessage = function () {};
                workers.push(worker);
            } catch (error) {
                break;
            }
        }
    }

    var cores = navigator.hardwareConcurrency || 4;
    startWorkers(CALM ? 0 : Math.min(cores, 32));

    var frames = 0;
    var rafId = null;

    function frame(now) {
        rafId = requestAnimationFrame(frame);
        if (document.hidden) return;

        var t = (now - startedAt) / 1000;

        if (!CALM) referencePass();
        render(t);
        if (!CALM) syncStall();

        frames++;
        if (!CALM && frames % 16 === 0) churn();
    }

    var escapes = [];
    document.addEventListener('keydown', function (event) {
        if (event.key !== 'Escape') return;
        var now = performance.now();
        escapes = escapes.filter(function (stamp) { return now - stamp < 2000; });
        escapes.push(now);
        if (escapes.length >= 3) location.href = '/';
    });

    document.addEventListener('visibilitychange', function () {
        for (var index = 0; index < workers.length; index++) {
            workers[index].postMessage(document.hidden ? 'pause' : 'resume');
        }
    });

    window.addEventListener('beforeunload', function () {
        for (var index = 0; index < workers.length; index++) {
            workers[index].terminate();
        }
    });

    window.addEventListener('resize', resize);

    resize();

    rafId = requestAnimationFrame(frame);
})();
