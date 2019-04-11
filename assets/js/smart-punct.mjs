
// Unicode fractions.
// Thanks, http://unicodefractions.com/
const mapping_fractions = {
    "1/2": "½",
    "1/3": "⅓",
    "2/3": "⅔",
    "1/4": "¼",
    "3/4": "¾",
    "1/5": "⅕",
    "2/5": "⅖",
    "3/5": "⅗",
    "4/5": "⅘",
    "1/6": "⅙",
    "5/6": "⅚",
    "1/7": "⅐",
    "1/8": "⅛",
    "3/8": "⅜",
    "5/8": "⅝",
    "7/8": "⅞",
    "1/9": "⅑",
    "1/10": "⅒",
};

function mapFraction(context, ss, next) {
    for (let fraction_ascii in mapping_fractions) {
        if (context.endsWith(fraction_ascii)) {
            const fraction_value = mapping_fractions[fraction_ascii];
            this.value = this.value.substr(0, ss - fraction_ascii.length) + fraction_value + next + this.value.substr(ss);
            this.setSelectionRange(ss - fraction_ascii.length + 2, ss - fraction_ascii.length + 2);
            return true;
        }
    }
    const matches = /[0-9]+\/[0-9]+$/.exec(context);
    if (matches !== null && matches.length !== 0) {
        const fraction_value = matches[0].replace("/", "\u2044");
        this.value = this.value.substr(0, ss - matches[0].length) + fraction_value + next + this.value.substr(ss);
        this.setSelectionRange(ss + 1, ss + 1);
        return true;
    }
    return false;
}

function keyDown(e) {
    const ss = this.selectionStart;
    const se = this.selectionEnd;
    if (ss !== undefined &&
        se !== undefined &&
        ss === se) {
        const cLen = ss >= 50 ? 50 : ss;
        const c = this.value.substr(ss < 50 ? 0 : ss - 50, cLen).split(/(\r|\n|\r\n)(\r|\n|\r\n)/);
        const context = c[c.length-1];
        let nextChar;
        let replaceChar;
        if (/({[\r\n]|\([\r\n]|<[a-zA-Z]|[a-zA-Z]\(|[a-zA-Z]\[)/.test(context)) {
            // looks like a function call foo(, an array index foo[, an html tag <foo,
            // or block brackets. These heuristics are done to avoid mangling computer code.
            // No attempt is made to detect URLs, both because these tend to mix with
            // plain English more than other computer code, and because URLs on their own
            // rarely contain quotes.
        } else if (e.key === '"') {
            const goodStartQuote = '“';
            const goodEndQuote = '”';
            const prime = '″';
            if (/\s$|\($|\[$/.test(context) || context === "") {
                // preceded by space or parens, or at the beginning of a paragraph.
                nextChar = goodStartQuote;
            } else if (/[0-9]$/.test(context) && !/“\w+$/.test(context)) {
                // [0-9] are the ASCII digits
                // When followed by an unmatched quote mark, these should be rendered as the
                // "inches" sign
                nextChar = prime;
                if (this.mapFraction(context, ss, prime)) {
                    e.preventDefault();
                    return;
                }
            } else {
                // otherwise, end quote
                nextChar = goodEndQuote;
            }
        } else if (e.key === "'") {
            const goodStartQuote = '‘';
            const goodEndQuote = '’';
            const prime = '′';
            if (/\s$|\($|\[$/.test(context) || context === "") {
                // preceded by space or parens, or at the beginning of a paragraph.
                nextChar = goodStartQuote;
            } else if (/[0-9]$/.test(context) && !/‘\w+$/.test(context)) {
                // [0-9] are the ASCII digits
                // When followed by an unmatched quote mark, these should be rendered as the
                // "inches" sign
                nextChar = prime;
                if (this.mapFraction(context, ss, prime)) {
                    e.preventDefault();
                    return;
                }
            } else {
                // otherwise, end quote
                nextChar = goodEndQuote;
            }
        } else if (e.key === '-') {
            const enDash = '–';
            const emDash = '—';
            if (/-$/.test(context)) {
                replaceChar = enDash;
            } else if (/–$/.test(context)) {
                replaceChar = emDash;
            }
        } else if (e.key === '.' && (context.endsWith("..") || context.endsWith(". . "))) {
            if (context.endsWith("..")) {
                this.value = this.value.substr(0, ss - 2) + '…' + this.value.substr(ss);
                this.setSelectionRange(ss - 1, se - 1);
                e.preventDefault();
                return;
            } else if (context.endsWith(". . ")) {
                this.value = this.value.substr(0, ss - 4) + '…' + this.value.substr(ss);
                this.setSelectionRange(ss - 3, se - 3);
                e.preventDefault();
                return;
            }
        } else if (/^\W$/.test(e.key) || e.key.toLowerCase() === 'enter' || e.key.toLowerCase() === 'return') {
            // whitelist of abbreviations that need apostrophes
            const results = /(^|\W)[‘’′'](tis|twas|cause|em|n[‘’′']|[0-9]+[a-zA-Z]+)$/i.exec(context);
            if (results !== null && results.length !== 0) {
                this.value = this.value.substr(0, ss - results[0].length) + results[1] + "’" + results[2].replace(/[‘’′]/, "’") + this.value.substr(ss);
                this.setSelectionRange(ss, se);
                return;
            } else {
                const next = (e.key.toLowerCase() === 'enter' || e.key.toLowerCase() === 'return') ? '\n' : e.key;
                if (this.mapFraction(context, ss, next)) {
                    e.preventDefault();
                    return;
                }
            }
        } else if (/\w[‘’′']$/.test(context)) {
            // abbreviations and possessives
            if (/^[a-zA-Z]$/.test(e.key)) {
                // special-case the '90's (and anything else that follows the 'blah're pattern
                const matches = /[‘’′'](\w+)[‘’′']$/.exec(context);
                if (matches !== null && matches.length !== 0) {
                    replaceChar = '’' + matches[1] + '’' + e.key;
                } else {
                    replaceChar = "’" + e.key;
                }
            }
        }
        if (nextChar !== undefined) {
            const start = this.value.substr(0, ss);
            const end = this.value.substr(ss);
            this.value = start + nextChar + end;
            this.setSelectionRange(ss + 1, se + 1);
            e.preventDefault();
        } else if (replaceChar !== undefined) {
            const start = this.value.substr(0, ss - replaceChar.length + 1);
            const end = this.value.substr(ss);
            this.value = start + replaceChar + end;
            this.setSelectionRange(ss + 1, ss + 1);
            e.preventDefault();
        }
    }
}

class SmartPunctInputElement extends HTMLInputElement {
    constructor() {
        super();
        this.addEventListener("keydown", keyDown);
        this.mapFraction = mapFraction;
    }
}

if (!window.customElements.get('smart-punct-input')) {
    window.SmartPunctInputElement = SmartPunctInputElement;
    window.customElements.define('smart-punct-input', SmartPunctInputElement, {
        extends: "input"
    });
}

class SmartPunctTextAreaElement extends HTMLTextAreaElement {
    constructor() {
        super();
        this.addEventListener("keydown", keyDown);
        this.mapFraction = mapFraction;
    }
}

if (!window.customElements.get('smart-punct-textarea')) {
    window.SmartPunctTextAreaElement = SmartPunctTextAreaElement;
    window.customElements.define('smart-punct-textarea', SmartPunctTextAreaElement, {
        extends: "textarea"
    });
}
