export class TagsTypeaheadElement extends HTMLInputElement {
    constructor() {
        super();
        this._data = null;
        this._menu = null;
        this._index = 0;
        if (window.fetch) {
            this.parentNode.setAttribute("role", "combobox");
            this.parentNode.setAttribute("aria-haspopup", "listbox");
            this.parentNode.setAttribute("aria-expanded", "false");
            this.setAttribute("aria-autocomplete", "list");
            this.addEventListener("click", this._change.bind(this));
            this.addEventListener("blur", this._blur.bind(this));
            this.addEventListener("keydown", this._keydown.bind(this));
            this.addEventListener("paste", this._change.bind(this));
            this.addEventListener("change", this._change.bind(this));
            this._dataType = this.getAttribute("data-type");
            if (!this._dataType) {
                this._dataType = "tag";
            }
            if (this.getAttribute("data-type") === "tag") {
                this._loadData();
            }
        }
    }
    _loadData() {
        var url, value;
        if (this._dataType === "tag") {
            url = "tags.json";
        } else if (this._dataType === "domain") {
            var tag_split = /[#, \t\|]+/;
            value = this.value.split(tag_split);
            if (value.length === 0) {
                return;
            }
            value = value[value.length - 1];
            value = value.replace(/^www\./, "");
            if (value.length >= 3) {
                value = this._hugeResults ? value : value.substr(0, 3);
                url = "domains.json?search=" + encodeURIComponent(value);
            } else {
                return;
            }
        } else {
            alert("Unexpected data-type=" + this._dataType);
            throw new Exception("Unexpected data-type=" + this._dataType);
        }
        fetch(url, {
            method: "get",
            credentials: "include"
        })
        .then(r => {
            if (r.ok) {
                return r.json();
            } else {
                window.log("failed to load " + this._dataType, r);
                return null;
            }
        })
        .then(json => {
            this._data = json;
            if (this === document.activeElement) {
                this._change();
            }
        }, e => {
            window.log("errored while loading " + this._dataType, e);
        });
    }
    _blur() {
        if (this._menu) {
            this._menu.style.opacity = "0";
        }
        setTimeout(() => {
            if (this === document.activeElement) {
                this._menu.style.opacity = "1";
                return;
            }
            if (this._menu) {
                this.parentNode.removeChild(this._menu);
            }
            this._menu = null;
            this._index = 0;
            this.parentNode.setAttribute("aria-expanded", "false");
        }, 100);
    }
    _keydown(e) {
        if (!this._data) {
            this._loadData();
            return;
        }
        if (this !== document.activeElement) {
            return;
        }
        switch (e.key.toLowerCase()) {
            case "escape":
            case "esc":
                if (this._menu) this.parentNode.removeChild(this._menu);
                this._menu = null;
                this._index = -2;
                this.parentNode.setAttribute("aria-expanded", "false");
                e.preventDefault();
                e.stopPropagation();
                break;
            case "arrowdown":
            case "down":
                if (!this._menu) {
                    // Arrow down when the menu is not open: open the menu.
                    this._index = 0;
                } else {
                    // Arrow down when an item other than the last is focused: focus next item.
                    // Arrow down when the last item is focused: jump to top (_change takes care of this).
                    // Both do this.
                    this._index += 1;
                }
                this._change();
                e.preventDefault();
                e.stopPropagation();
                break;
            case "pgdb":
            case "pagedown":
                this._index += 4;
                var availableTags = Object.keys(this._data).filter(tag => {
                    return !currentTag || tag.indexOf(currentTag) !== -1;
                });
                if (this._index >= availableTags.length) {
                    this._index = availableTags.length - 1;
                }
                e.preventDefault();
                e.stopPropagation();
                this._change();
                break;
            case "pgup":
            case "pageup":
                this._index -= 4;
                if (this._index <= 0) {
                    this._index = 0;
                }
                e.preventDefault();
                e.stopPropagation();
                this._change();
                break;
            case "arrowup":
            case "up":
                this._index -= 1;
                e.preventDefault();
                e.stopPropagation();
                this._change();
                break;
            case "enter":
            case "return":
            case "tab":
                if (!this._menu) return;
                var tag_split_last = /([#, \t\|])(?!.*[#, \t\|])/g;
                var tag_split = /[#, \t\|]+/g;
                var currentTagParts = this.value.split(tag_split);
                var currentTag = currentTagParts.length > 1 ? currentTagParts[currentTagParts.length-1] : this.value;
                var availableTags = Object.keys(this._data).filter(tag => {
                    return !currentTag || tag.indexOf(currentTag) !== -1;
                });
                availableTags.sort(function(a, b) {
                    if (a[0] < 'a' && b[0] >= 'a') return 1;
                    if (b[0] < 'a' && a[0] >= 'a') return -1;
                    if (a < b) return -1;
                    if (b < a) return 1;
                    return 0;
                });
                if (tag_split_last.test(this.value)) {
                    this.value = this.value.slice(0, tag_split_last.lastIndex) + availableTags[this._index] + " ";
                } else {
                    this.value = availableTags[this._index] + " ";
                }
                if (e.key.toLowerCase() === "tab") {
                    this._index = -2;
                } else {
                    this._index = 0;
                }
                this._change();
                e.preventDefault();
                e.stopPropagation();
                break;
            case "shift":
                break;
            default:
                if (this._index === -2) this._index = 0;
                setTimeout(() => { this._change(); }, 1);
        }
    }
    _change() {
        if (!this._data) {
            this._loadData();
            return;
        }
        if (this !== document.activeElement) {
            return;
        }
        var tag_split = /[#, \t\|]+/;
        var tag_split_last = /([#, \t\|])(?!.*[#, \t\|])/g;
        var currentTagParts = this.value.split(tag_split);
        var currentTag = currentTagParts.length > 1 ? currentTagParts[currentTagParts.length-1] : this.value;
        if (this._dataType === "domain" && currentTag.length < 3) {
            this._data = null;
            this.parentNode.removeChild(this._menu);
            this._menu = null;
            return;
        }
        if (this._menu) {
            this.parentNode.removeChild(this._menu);
            this._menu = null;
        }
        if (this._index === -2) {
            return;
        }
        var availableTags = Object.keys(this._data).filter(tag => {
            return !currentTag || tag.indexOf(currentTag) !== -1;
        });
        if (this._hugeResults || (availableTags.length > 500 && this._dataType === "domain")) {
            this._loadData();
            this._hugeResults = true;
        }
        this.parentNode.setAttribute("aria-expanded", "true");
        this.setAttribute("aria-controls", "tags-typeahead");
        this.setAttribute("autocomplete", "off");
        this._menu = document.createElement("div");
        this._menu.className = "typeahead-inner";
        this._menu.setAttribute("role", "listbox");
        this._menu.id = "tags-typeahead";
        this._menu.setAttribute("tabindex", "-1");
        this.parentNode.appendChild(this._menu);
        availableTags.sort(function(a, b) {
            if (a[0] < 'a' && b[0] >= 'a') return 1;
            if (b[0] < 'a' && a[0] >= 'a') return -1;
            if (a < b) return -1;
            if (b < a) return 1;
            return 0;
        });
        if (this._index + 1 > availableTags.length) {
            this._index = 0;
        }
        if (this._index < 0) {
            this._index = availableTags.length - 1;
        }
        var self = this;
        var i;
        for (i in availableTags) {
            var tag = availableTags[i];
            var menuItem = document.createElement("button");
            menuItem.type = "button";
            menuItem.setAttribute("role", "option");
            menuItem.className = "details-menu-item" + (i == this._index ? ' details-menu-item-active' : '');
            menuItem.setAttribute("aria-selected", (i == this._index) ? "true" : "false");
            menuItem.id = "tags-typeahead-" + i;
            menuItem.appendChild(document.createTextNode(tag));
            menuItem._tag = tag;
            menuItem._index = i;
            menuItem.setAttribute("tabindex", "-1");
            menuItem.onmouseover = function() {
                if (self._index != this._index) {
                    var old = self._menu.getElementsByClassName("details-menu-item-active")[0];
                    old.setAttribute("aria-selected", "false");
                    old.className = "details-menu-item";
                    self._index = this._index;
                    this.className += ' details-menu-item-active';
                    this.setAttribute("aria-selected", "true");
                    self.setAttribute("aria-activedescendant", "tags-typeahead-" + this._index);
                }
            };
            menuItem.ontouchstart = function() {
                delete this.onmousedown;
            };
            menuItem.onclick = menuItem.onmousedown = function() {
                if (self.value.endsWith(this._tag + " ")) {
                    // do nothing
                } else if (tag_split_last.test(self.value)) {
                    self.value = self.value.slice(0, tag_split_last.lastIndex) + this._tag + " ";
                } else {
                    self.value = this._tag + " ";
                }
                if (self._menu) self.parentNode.removeChild(self._menu);
                self._menu = null;
                self._index = 0;
                self.parentNode.setAttribute("aria-expanded", "false");
                return false;
            };
            this._menu.appendChild(menuItem);
            if (i == this._index) {
                var scrollBottom = this._menu.clientHeight + this._menu.scrollTop;
                var elementBottom = menuItem.offsetTop + menuItem.offsetHeight;
                if (elementBottom > scrollBottom) {
                  this._menu.scrollTop = elementBottom - menuItem.clientHeight;
                } else if (menuItem.offsetTop < this._menu.scrollTop) {
                  this._menu.scrollTop = menuItem.offsetTop;
                }
            }
        }
        if (availableTags.length === 0) {
            this._menu.innerHTML = "<span class=details-menu-item>No " + this._dataType + " found</span>";
        }
        this.setAttribute("aria-activedescendant", "tags-typeahead-" + this._index);
    }
}

if (!window.customElements.get('tags-typeahead')) {
    window.TagsTypeaheadElement = TagsTypeaheadElement;
    window.customElements.define('tags-typeahead', TagsTypeaheadElement, {
        extends: "input"
    });
}
