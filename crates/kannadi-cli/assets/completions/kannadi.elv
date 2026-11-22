
use builtin;
use str;

set edit:completion:arg-completer[kannadi] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'kannadi'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'kannadi'= {
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
            cand to-unicode 'Convert documents in legacy Kannada fonts to Unicode'
            cand to-nudi 'Convert Unicode Kannada documents to Nudi glyph codes'
            cand normalize 'Normalize Unicode Kannada text, following the Kannada text normalization profile'
            cand type 'Show the Kannada words for words typed in English letters, best first'
            cand completions 'Print a shell completion script to standard output'
            cand manpages 'Write the man pages to a folder'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'kannadi;to-unicode'= {
            cand -o 'Folder to write the converted files and reports to'
            cand --out 'Folder to write the converted files and reports to'
            cand --layout 'Layout of text files, and of fonts added with --legacy-font that name none. nudi covers the Nudi and Baraha ANSI fonts'
            cand --font 'Font to set converted text in, in HTML and DOCX files'
            cand --legacy-font 'Treat this font as a legacy font too, in the layout after `=` ("My Font=akruti") or else the --layout one. Repeat for more fonts'
            cand --strict 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count'
            cand --kannada-digits 'Write digits as Kannada digits, for text set in the k variants of the Nudi fonts'
            cand --keep-zeros 'Keep a zero typed between letters as a digit; by default it becomes ಂ'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;to-nudi'= {
            cand -o 'Folder to write the converted files and reports to'
            cand --out 'Folder to write the converted files and reports to'
            cand --font 'Font to set converted text in, in HTML and DOCX files'
            cand --legacy-font 'Treat this font as a legacy font too, so text already set in it is left as it is. Repeat for more fonts'
            cand --strict 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;normalize'= {
            cand --form 'How far to go'
            cand --stats 'Print how many times each rule changed the text to standard error, as TSV'
            cand --check 'Write nothing; exit with status 1 if any input is not already in the form'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;type'= {
            cand -n 'How many words to show for each'
            cand --prev 'The Kannada word before, which can change the order'
            cand --scores 'Show 1 word per line, with its score and kind: word, spelling (not in the word list) or completion'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;completions'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;manpages'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
        }
        &'kannadi;help'= {
            cand to-unicode 'Convert documents in legacy Kannada fonts to Unicode'
            cand to-nudi 'Convert Unicode Kannada documents to Nudi glyph codes'
            cand normalize 'Normalize Unicode Kannada text, following the Kannada text normalization profile'
            cand type 'Show the Kannada words for words typed in English letters, best first'
            cand completions 'Print a shell completion script to standard output'
            cand manpages 'Write the man pages to a folder'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'kannadi;help;to-unicode'= {
        }
        &'kannadi;help;to-nudi'= {
        }
        &'kannadi;help;normalize'= {
        }
        &'kannadi;help;type'= {
        }
        &'kannadi;help;completions'= {
        }
        &'kannadi;help;manpages'= {
        }
        &'kannadi;help;help'= {
        }
    ]
    $completions[$command]
}
