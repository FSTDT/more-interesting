var assert = require("assert");
const chrome = require('selenium-webdriver/chrome');
const firefox = require('selenium-webdriver/firefox');
const webdriver = require('selenium-webdriver');
const {Builder, By, Key, until} = webdriver;

const screen = {
    width: 640,
    height: 480
};

describe("testing javascript in the browser", function() {
    beforeEach(async function() {
        this.browser = await new Builder()
            .forBrowser(process.env.MORE_INTERESTING_WEBDRIVER || "firefox")
            .setChromeOptions(new chrome.Options().headless().windowSize(screen))
            .setFirefoxOptions(new firefox.Options().headless().windowSize(screen))
            .build();
        await this.browser.get("http://localhost:3001/-setup");
        await this.browser.wait(until.urlIs("http://localhost:3001/-setup"));
        await this.browser.get("http://localhost:3001/-login");
        await this.browser.wait(until.elementLocated(By.css('input[type="password"]')));
        var username = await this.browser.findElement(By.css('input'));
        await username.sendKeys("root");
        var password = await this.browser.findElement(By.css('input[type="password"]'));
        await password.sendKeys("ready2go");
        var button = await this.browser.findElement(By.css('form[action=""] button'));
        await button.click();
        await this.browser.wait(until.urlIs("http://localhost:3001/"));
    });

    afterEach(function() {
        return this.browser.quit();
    });

    it("first link on submit page should be Home", async function() {
        await this.browser.get("http://localhost:3001/-submit");
        await this.browser.wait(until.urlIs("http://localhost:3001/-submit"));
        var headline = await this.browser.findElement(By.css('a'));
        var text = await headline.getText();
        return assert.equal(text, "Home");
    });

    it("log out button should go away when you log out", async function() {
        var logout_button = await this.browser.findElement(By.css('button[formaction="-logout"]'));
        await logout_button.click();
        await this.browser.wait(until.urlIs("http://localhost:3001/"));
        logout_button = await this.browser.findElements(By.css('button[formaction="-logout"]'));
        return assert(!logout_button.length);
    });
});
