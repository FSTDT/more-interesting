module.exports = [{
    output: {
        filename: './modern.min.js',
    },
    devtool: "source-map",
    name: 'modern',
    entry: './js/modern.mjs',
    mode: 'production'
    // note: this mode does not, and CANNOT, use Babel.
    // Babel would translate the ES6 classes into plain ES5 constructors,
    // but that would break running under native customElements.
}, {
    output: {
        filename: './legacy.min.js',
    },
    devtool: "source-map",
    name: 'legacy',
    entry: './js/legacy.mjs',
    mode: 'production',
    module: {
        rules: [
            {
                test: /\.m?js$/,
                use: {
                    loader: 'babel-loader',
                    options: {
                        presets: [['@babel/preset-env', {
                            targets: {
                                "ie": "11"
                            }
                        }]]
                    }
                }
            }
        ]
    }
}];
