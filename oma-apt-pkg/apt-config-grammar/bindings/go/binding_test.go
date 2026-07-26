package tree_sitter_apt_config_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_apt_config "github.com/tree-sitter/tree-sitter-apt_config/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_apt_config.Language())
	if language == nil {
		t.Errorf("Error loading AptConfig grammar")
	}
}
