define([
    "require",
    "exports",
    "white",
    "black"
], function(require, exports, _white, _black) {
    "use strict";
    Object.defineProperty(exports, "__esModule", {
        value: true
    });
    function _export(target, all) {
        for(var name in all)Object.defineProperty(target, name, {
            get: all[name],
            enumerable: true
        });
    }
    _export(exports, {
        named1: ()=>_white.named1,
        named2: ()=>_black.named2
    });
});
