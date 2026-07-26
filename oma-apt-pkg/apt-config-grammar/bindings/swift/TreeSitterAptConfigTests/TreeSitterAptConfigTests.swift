import XCTest
import SwiftTreeSitter
import TreeSitterAptConfig

final class TreeSitterAptConfigTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_apt_config())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading AptConfig grammar")
    }
}
