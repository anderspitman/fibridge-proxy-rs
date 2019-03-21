const HtmlWebpackPlugin = require('html-webpack-plugin');
const HtmlWebpackInlineSourcePlugin = require('html-webpack-inline-source-plugin');

module.exports = {
  mode: 'development',
  plugins: [
    new HtmlWebpackPlugin({
      // embed all javascript and css inline in HTML file
      inlineSource: '.(js|css)$'
    }),
    new HtmlWebpackInlineSourcePlugin()
  ],
};
