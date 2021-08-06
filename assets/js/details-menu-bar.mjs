function firstVisibleSibling(el) {
    while (el && !el.offsetHeight) {
        el = el.nextElementSibling;
    }
    return el;
}

function lastVisibleSibling(el) {
    while (el && !el.offsetHeight) {
        el = el.previousElementSibling;
    }
    return el;
}

function firstVisibleChild(el) {
    return firstVisibleSibling(el.firstElementChild);
}

function lastVisibleChild(el) {
    return lastVisibleSibling(el.lastElementChild);
}

function isChildOf(el, parent) {
    while (el) {
        if (el == parent) {
            return true;
        }
        el = el.parentNode;
    }
    return false;
}

document.querySelector("body").classList.add("js");

let currentlyOpen = null;

export class DetailsMenuBarElement extends HTMLElement {
    constructor() {
        super();
        let details = this.querySelectorAll(".details-menu-outer");
        var summary;
        for (let d of details) {
            summary = d.querySelector("summary");
            d.addEventListener("keydown", this._eventKeydown.bind(d));
            d.addEventListener("mouseover", this._eventMouseOver.bind(d));
            d.addEventListener("toggle", this._eventToggle.bind(d));
            d.querySelector(".details-menu-inner").addEventListener("mouseleave", this._eventMouseLeave.bind(d));
            for (let m of d.querySelectorAll(".details-menu-item")) {
                m.addEventListener("click", this._eventMouseClick.bind(d));
                m.addEventListener("mouseup", this._eventRelease.bind(d));
            }
            summary.addEventListener("mousedown", this._eventTopClick.bind(d));
            summary.addEventListener("touchstart", this._eventTouchStart.bind(d));
            summary.addEventListener("touchmove", this._eventTouchMove.bind(d));
            summary.addEventListener("touchend", this._eventTouchEnd.bind(d));
            summary.addEventListener("click", function(e) { e.preventDefault() });
            summary.addEventListener("mouseup", this._eventRelease.bind(d));
        }
    }
    _eventTopClick(e) {
        e.preventDefault();
        e.stopPropagation();
        if (!this._touched) {
            this.open = !this.open;
        }
    }
    _eventTouchStart(e) {
        this.open = !this.open;
        this._touched = true;
        if (e.touches && e.touches[0] && e.touches[0].touchType === "stylus" && this.open) {
            e.preventDefault();
        }
    }
    _eventTouchMove(e) {
        // Treat a stylus like a mouse
        if (e.touches && e.touches[0] && e.touches[0].touchType === "stylus" && this.open) {
            this._moved = false;
            this._stylus = true;
            var el = document.elementFromPoint(e.touches[0].clientX, e.touches[0].clientY);
            if (isChildOf(el, this)) {
                el.focus();
            }
            e.preventDefault();
        } else {
            delete this._stylus;
            this._moved = true;
            this._touched = true;
            this.open = false;
        }
    }
    _eventTouchEnd(e) {
        e.preventDefault();
        delete this._touched;
        if (this._stylus) {
            delete this._stylus;
            var el = this.querySelector(":focus");;
            var summary = this.querySelector("summary");
            if (el !== summary) {
                el.click();
                this.open = false;
            }
        }
        if (this._moved) {
            delete this._moved;
            this.open = false;
        }
    }
    _eventRelease(e) {
        var summary = this.querySelector("summary");
        if (e.target !== summary && !e.shiftKey && !e.metaKey && !e.ctrlKey && !e.altKey && e.button === 0) {
            e.currentTarget.click();
            e.preventDefault();
            e.stopPropagation();
        }
    }
    // keyboard behavior for these menus should behave at the intersection of the menus on GitHub and the WAI ARIA menus
    // See https://www.w3.org/TR/wai-aria-practices-1.1/examples/menubar/menubar-1/menubar-1.html for more info on a lot of these practices.
    // Key word: principle of least surprise.
    _eventKeydown(e) {
        let inner = this.querySelector(".details-menu-inner");
        let current = inner.querySelector(":focus");
        let switchTo = null;
        switch (e.key.toLowerCase()) {
            case "escape":
            case "esc":
                // Escape closes the menu. GitHub and WAI both do this.
                this.open = false;
                e.preventDefault();
                e.stopPropagation();
                break;
            case "arrowdown":
            case "down":
                if (e.target.className.indexOf("menu-summary") !== -1 && !this.open) {
                    // Arrow down when a menu button is selected: open the menu.
                    // WAI does this, GitHub does not.
                    // The most likely scenario where a user expecting GitHub's behavior would accidentally
                    // trigger this is if they were trying to scroll, but it's more likely they would either have
                    // nothing focused or they'd have a link below this area focused.
                    this.open = true;
                    firstVisibleChild(inner).focus();
                } else if (!current || current.className !== "details-menu-item") {
                    // Arrow down when a menu is open and nothing is focused: focus first item.
                    // Both do this.
                    firstVisibleChild(inner).focus();
                } else {
                    // Arrow down when an item other than the last is focused: focus next item.
                    // Arrow down when the last item is focused: jump to top.
                    // Both do this.
                    (firstVisibleSibling(current.nextElementSibling) || firstVisibleChild(inner)).focus();
                }
                e.preventDefault();
                e.stopPropagation();
                break;
            case "arrowup":
            case "up":
                if (e.target.className.indexOf("menu-summary") !== -1 && !this.open) {
                    // Arrow up when a menu button is selected: open the menu.
                    // WAI does this, GitHub does not.
                    // The most likely scenario where a user expecting GitHub's behavior would accidentally
                    // trigger this is if they were trying to scroll, but it's more likely they would either have
                    // nothing focused or they'd have a link below this area focused.
                    this.open = true;
                    lastVisibleChild(inner).focus();
                } else if (!current || current.className !== "details-menu-item") {
                    // Arrow up when a menu is open and nothing is focused: focus last item.
                    // Both do this.
                    lastVisibleChild(inner).focus();
                } else {
                    // Arrow up when an item other than the first is focused: focus previous item.
                    // Arrow up when the first item is focused: jump to bottom.
                    // Both do this.
                    (lastVisibleSibling(current.previousElementSibling) || lastVisibleChild(inner)).focus();
                }
                e.preventDefault();
                e.stopPropagation();
                break;
            case "arrowleft":
            case "left":
                if (this.open) {
                    // left arrow: switch to previous menu, or switch to last
                    // this only occurs when the menu is already opened,
                    // and there's no other logical behavior for it to have,
                    // so users expecting GitHub's behavior will probably not accidentally trigger it
                    if (this === firstVisibleChild(this.parentNode)) {
                        switchTo = lastVisibleChild(this.parentNode);
                    } else {
                        switchTo = lastVisibleSibling(this.previousElementSibling) || lastVisibleChild(this.parentNode);
                    }
                }
                break;
            case "arrowright":
            case "right":
                if (this.open) {
                    // right arrow: switch to next menu, or switch to first
                    // this only occurs when the menu is already opened,
                    // and there's no other logical behavior for it to have,
                    // so users expecting GitHub's behavior will probably not accidentally trigger it
                    if (this === lastVisibleChild(this.parentNode)) {
                        switchTo = firstVisibleChild(this.parentNode);
                    } else {
                        switchTo = firstVisibleSibling(this.nextElementSibling) || firstVisibleChild(this.parentNode);
                    }
                }
                break;
            case "tab":
                if (!this.open) {
                    // if the menu is not open, tab should have no special behavior
                    // this is a willful violation of the WAI behavior, in favor of emulating GitHub,
                    // because if a user doesn't know about the left-right arrow trigger,
                    // they might not know how to switch menus at all.
                    break;
                } else if (!current || current.className !== "details-menu-item") {
                    // if the menu is open, we should focus trap into it
                    // this is the behavior of the WAI example
                    // it is not the same as GitHub's example, but GitHub allows you to tab yourself out
                    // of the menu without closing it (which is horrible behavior)
                    (e.shiftKey ? lastVisibleChild(inner) : firstVisibleChild(inner)).focus();
                    e.preventDefault();
                    e.stopPropagation();
                } else if (e.shiftKey && current === firstVisibleChild(inner)) {
                    // if you tab your way out of the menu, close it
                    // this is neither what GitHub nor the WAI example do,
                    // but is a rationalization of GitHub's behavior: we don't want users who know how to
                    // use tab and enter, but don't know that they can close menus with Escape,
                    // to find themselves completely trapped in the menu
                    lastVisibleChild(inner).focus();
                    this.open = false;
                } else if (!e.shiftKey && current === lastVisibleChild(inner)) {
                    // same as above.
                    // if you tab your way out of the menu, close it
                    this.open = false;
                }
                break;
            case "enter":
            case "return":
                // enter, return, and space have the default browser behavior,
                // but they also close the menu
                // this behavior is identical between both the WAI example, and GitHub's
                setTimeout(function() {
                    this.open = false;
                }, 100);
                break;
            case "space":
            case " ":
                // space closes the menu, and activates the current link
                // this behavior is identical between both the WAI example, and GitHub's
                if (document.activeElement instanceof HTMLAnchorElement || document.activeElement instanceof HTMLButtonElement) {
                    // It's supposed to copy the behaviour of the WAI Menu Bar
                    // page, and of GitHub's menus. I've been using these two
                    // sources to judge what is basically "industry standard"
                    // behaviour for menu keyboard activity on the web.
                    //
                    // On GitHub, here's what I notice:
                    //
                    // 1 If you click open a menu, the menu button remains
                    //   focused. If, in this stage, I press space, the menu will
                    //   close.
                    //
                    // 2 If I use the arrow keys to focus a menu item, and then
                    //   press space, the menu item will be activated. For
                    //   example, clicking "+", then pressing down, then pressing
                    //   space will open the New Repository page.
                    //
                    // Behaviour 1 is why the
                    // `!document.activeElement.hasAttribute("aria-haspopup")`
                    // condition is there. It's to make sure the menu-link on
                    // things like the About dropdown don't get activated.
                    // Behaviour 2 is why this code is required at all; I want to
                    // activate the currently highlighted menu item.
                    document.activeElement.click();
                }
                setTimeout(function() {
                    this.open = false;
                }, 100);
                e.preventDefault();
                e.stopPropagation();
                break;
            case "home":
                if (this.open) {
                    // home: focus first menu item.
                    // This is the behavior of WAI, while GitHub scrolls,
                    // but it's unlikely that a user will try to scroll the page while the menu is open,
                    // so they won't do it on accident.
                    firstVisibleChild(inner).focus();
                    e.preventDefault();
                    e.stopPropagation();
                }
                break;
            case "end":
                if (this.open) {
                    // end: focus last menu item.
                    // This is the behavior of WAI, while GitHub scrolls,
                    // but it's unlikely that a user will try to scroll the page while the menu is open,
                    // so they won't do it on accident.
                    lastVisibleChild(inner).focus();
                    e.preventDefault();
                    e.stopPropagation();
                }
                break;
            default:
                // letter and number keys will focus the menu item that starts with that letter.
                // This is the behavior of WAI: GitHub does nothing.
                // Users are unlikely to accidentally type words while the menu is open,
                // so they're unlikely to trigger the behavior by mistake.
                if (this.open && firstVisibleChild(inner)) {
                    let focused = firstVisibleSibling(current ? current.nextElementSibling : null) || firstVisibleChild(inner);
                    while (focused !== current && !focused.innerText.toLocaleLowerCase().startsWith(e.key.toLocaleLowerCase())) {
                        focused = firstVisibleSibling(focused.nextElementSibling);
                        if (!focused) {
                            focused = firstVisibleChild(inner);
                            if (!current) {
                                break;
                            }
                        }
                    }
                    if (focused && focused.innerText.toLocaleLowerCase().startsWith(e.key.toLocaleLowerCase())) {
                        focused.focus();
                        e.preventDefault();
                        e.stopPropagation();
                    }
                }
        }
        if (switchTo) {
            switchTo.open = true;
            setTimeout(function() {
                switchTo.querySelector(".details-menu-item").focus();
            }, 10);
            e.preventDefault();
            e.stopPropagation();
        }
    }
    _eventMouseOver(e) {
        // keyboard focus should follow mouse focus
        // this is what WAI does
        // it is not what GitHub does, but GitHub's behavior permits a focus indicator
        // and a mouse indicator that are separate but look exactly the same
        if (e.target instanceof HTMLElement && (e.target.className === "details-menu-item" || e.target.parentNode.className === "details-menu-item")) {
            e.target.focus();
        }
    }
    _eventMouseLeave(e) {
        // all menu items should be unfocused when the mouse leaves the menu
        // this is what WAI does
        // it is not what GitHub does, but as I said above, GitHub's behavior is stupid in this case
        if (this.open) {
            this.querySelector("summary").focus();
        }
    }
    _eventMouseClick(e) {
        // when the user clicks a menu item, close the menu
        if (this.open) {
            this.open = false;
        }
    }
    _eventToggle(e) {
        // only one menu should ever be open at a time
        // this is very similar to what WAI does, but WAI does this tracking on a per-bar level
        // while More Interesting's behavior is page-global
        // GitHub's behavior also globally prevents you from opening more than one menu,
        // but they require two clicks to switch between them, while this implementation only requires one
        if (currentlyOpen && currentlyOpen !== this && this.open) {
            currentlyOpen.open = false;
        }
        this.querySelector("summary").setAttribute("aria-expanded", this.open ? "true" : "false");
        this.querySelector(".details-menu-inner").setAttribute("aria-hidden", this.open ? "false" : "true");
        setTimeout( () => {
            if (this.open) {
                currentlyOpen = this;
                if (!this.querySelector(":focus")) {
                    this.querySelector("summary").focus();
                }
            }
        });
    }
}

if (!window.customElements.get('details-menu-bar')) {
    window.DetailsMenuBarElement = DetailsMenuBarElement;
    window.customElements.define('details-menu-bar', DetailsMenuBarElement);
}
