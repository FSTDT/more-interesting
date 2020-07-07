export class TagsTypeaheadElement extends HTMLInputElement {
    constructor() {
        super();
        this._tags = null;
        this._menu = null;
        this._index = 0;
        if (window.fetch) {
            this.parentNode.setAttribute("role", "combobox");
            this.parentNode.setAttribute("aria-haspopup", "listbox");
            this.parentNode.setAttribute("aria-expanded", "false");
            this.setAttribute("aria-autocomplete", "list");
            this.addEventListener("focus", this._change.bind(this));
            this.addEventListener("blur", this._blur.bind(this));
            this.addEventListener("keydown", this._keydown.bind(this));
            this.addEventListener("keyup", this._change.bind(this));
            this.addEventListener("paste", this._change.bind(this));
            this.addEventListener("change", this._change.bind(this));
            fetch("tags.json", {
                method: "get",
                credentials: "include"
            })
            .then(r => {
                if (r.ok) {
                    return r.json();
                } else {
                    window.log("failed to load tags", r);
                    return null;
                }
            })
            .then(json => {
                this._tags = json;
                if (this === document.activeElement) {
                    this._change();
                }
            }, e => {
                window.log("errored while loading tags", e);
            });
        }
    }
    _blur() {
        if (this._menu) {
            this._menu.style.opacity = "0";
        }
        setTimeout(() => {
            if (this === document.activeElement) {
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
        if (!this._tags || this !== document.activeElement) {
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
                e.preventDefault();
                e.stopPropagation();
                break;
            case "arrowup":
            case "up":
                this._index -= 1;
                e.preventDefault();
                e.stopPropagation();
                break;
            case "enter":
            case "return":
                var tag_split_last = /[#, \t]+$/g;
                var currentTagParts = this.value.split(tag_split);
                var currentTag = currentTagParts.length > 1 ? currentTagParts[currentTagParts.length-1] : this.value;
                var availableTags = Object.keys(this._tags).filter(tag => {
                    return !currentTag || tag.indexOf(currentTag) !== -1;
                });
                if (this._index + 1 > availableTags.length) {
                    this._index = 0;
                }
                if (this._index < 0) {
                    this._index = avaiableTags.length - 1;
                }
                if (tag_split_last.test(this.value)) {
                    this.value = this.value.slice(0, tag_split_last.lastIndex) + availableTags[this._index] + " ";
                } else {
                    this.value = availableTags[this._index] + " ";
                }
                e.preventDefault();
                e.stopPropagation();
                break;
        }
    }
    _change() {
        if (!this._tags || this !== document.activeElement) {
            return;
        }
        if (this._index === -2) {
            return;
        }
        if (this._menu) {
            this.parentNode.removeChild(this._menu);
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
        var tag_split = /[#, \t]+/;
        var tag_split_last = /[#, \t]+$/g;
        var currentTagParts = this.value.split(tag_split);
        var currentTag = currentTagParts.length > 1 ? currentTagParts[currentTagParts.length-1] : this.value;
        var availableTags = Object.keys(this._tags).filter(tag => {
            return !currentTag || tag.indexOf(currentTag) !== -1;
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
            menuItem.onclick = function() {
                if (tag_split_last.test(self.value)) {
                    self.value = self.value.slice(0, tag_split_last.lastIndex) + this._tag + " ";
                } else {
                    self.value = this._tag + " ";
                }
                self.focus();
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
            this._menu.innerHTML = "<span class=details-menu-item>No tags found</span>";
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
