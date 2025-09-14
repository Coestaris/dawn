const CopyPlugin = require("copy-webpack-plugin");
const FaviconsWebpackPlugin = require("favicons-webpack-plugin");
const path = require("path");

module.exports = {
    entry: "./index.js",
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "index.js",
    },
    mode: "development",
    experiments: {
        asyncWebAssembly: true,
    },
    plugins: [
        new CopyPlugin({
            patterns: [{ from: "index.html" }],
        }),
        new FaviconsWebpackPlugin("../../../assets/icon.ico"),
    ],
};