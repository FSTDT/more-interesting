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
        if (!this.moreInterestingHasCode && context.indexOf("<code>") !== -1) {
            this.moreInterestingHasCode = true;
        }
        if (this.moreInterestingHasCode) {
            let bracketCount = 0;
            let endBracketCount = 0;
            let inCode = false;
            for (let k = 0; k !== ss; ++k) {
                if (this.value[k] === '<') {
                    bracketCount += 1;
                } else if (!inCode && bracketCount !== 0 && this.value[k] === 'c') {
                    if (this.value[k+1] === 'o' && this.value[k+2] === 'd' && this.value[k+3] === 'e') {
                        endBracketCount = 0;
                        for (let m = k+4; m !== ss; ++m) {
                            if (this.value[m] === '>') {
                                endBracketCount += 1;
                            } else {
                                break;
                            }
                            if (endBracketCount === bracketCount) {
                                break;
                            }
                        }
                        if (endBracketCount === bracketCount) {
                            inCode = true;
                        }
                    }
                    bracketCount = 0;
                } else if (inCode && bracketCount >= endBracketCount && this.value[k] === '/') {
                    if (this.value[k+1] === 'c' && this.value[k+2] === 'o' && this.value[k+3] === 'd' && this.value[k+4] === 'e') {
                        endBracketCount = 0;
                        for (let m = k+5; m !== ss; ++m) {
                            if (this.value[m] === '>') {
                                endBracketCount += 1;
                            } else {
                                break;
                            }
                            if (endBracketCount === bracketCount) {
                                break;
                            }
                        }
                        if (endBracketCount === bracketCount) {
                            inCode = false;
                        } else {
                            endBracketCount = bracketCount;
                        }
                        bracketCount = 0;
                    }
                }
            }
            if (inCode) {
                return;
            }
        }
        if (e.key === '"') {
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
            const resultsEndQuote = / +[“”"]$/.exec(context);
            if (results !== null && results.length !== 0) {
                this.value = this.value.substr(0, ss - results[0].length) + results[1] + "’" + results[2].replace(/[‘’′]/, "’") + this.value.substr(ss);
                this.setSelectionRange(ss, se);
                return;
            } else if (resultsEndQuote !== null && resultsEndQuote.length !== 0 && (e.key.toLowerCase() === 'enter' || e.key.toLowerCase() === 'return')) {
                this.value = this.value.substr(0, ss - resultsEndQuote[0].length) + "”\n" + this.value.substr(ss);
                this.setSelectionRange(ss, se);
                e.preventDefault();
                return;
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
        } else if (replaceChar !== undefined && replaceChar.length === 1) {
            const start = this.value.substr(0, ss - replaceChar.length);
            const end = this.value.substr(ss);
            this.value = start + replaceChar + end;
            this.setSelectionRange(ss, ss);
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
    }
}

if (!window.customElements.get('smart-punct-textarea')) {
    window.SmartPunctTextAreaElement = SmartPunctTextAreaElement;
    window.customElements.define('smart-punct-textarea', SmartPunctTextAreaElement, {
        extends: "textarea"
    });
}
