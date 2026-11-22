
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'kannadi' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'kannadi'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'kannadi' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('to-unicode', 'to-unicode', [CompletionResultType]::ParameterValue, 'Convert documents in legacy Kannada fonts to Unicode')
            [CompletionResult]::new('to-nudi', 'to-nudi', [CompletionResultType]::ParameterValue, 'Convert Unicode Kannada documents to Nudi glyph codes')
            [CompletionResult]::new('normalize', 'normalize', [CompletionResultType]::ParameterValue, 'Normalize Unicode Kannada text, following the Kannada text normalization profile')
            [CompletionResult]::new('type', 'type', [CompletionResultType]::ParameterValue, 'Show the Kannada words for words typed in English letters, best first')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Print a shell completion script to standard output')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Write the man pages to a folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kannadi;to-unicode' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Folder to write the converted files and reports to')
            [CompletionResult]::new('--out', '--out', [CompletionResultType]::ParameterName, 'Folder to write the converted files and reports to')
            [CompletionResult]::new('--layout', '--layout', [CompletionResultType]::ParameterName, 'Layout of text files, and of fonts added with --legacy-font that name none. nudi covers the Nudi and Baraha ANSI fonts')
            [CompletionResult]::new('--font', '--font', [CompletionResultType]::ParameterName, 'Font to set converted text in, in HTML and DOCX files')
            [CompletionResult]::new('--legacy-font', '--legacy-font', [CompletionResultType]::ParameterName, 'Treat this font as a legacy font too, in the layout after `=` ("My Font=akruti") or else the --layout one. Repeat for more fonts')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count')
            [CompletionResult]::new('--kannada-digits', '--kannada-digits', [CompletionResultType]::ParameterName, 'Write digits as Kannada digits, for text set in the k variants of the Nudi fonts')
            [CompletionResult]::new('--keep-zeros', '--keep-zeros', [CompletionResultType]::ParameterName, 'Keep a zero typed between letters as a digit; by default it becomes ಂ')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;to-nudi' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Folder to write the converted files and reports to')
            [CompletionResult]::new('--out', '--out', [CompletionResultType]::ParameterName, 'Folder to write the converted files and reports to')
            [CompletionResult]::new('--font', '--font', [CompletionResultType]::ParameterName, 'Font to set converted text in, in HTML and DOCX files')
            [CompletionResult]::new('--legacy-font', '--legacy-font', [CompletionResultType]::ParameterName, 'Treat this font as a legacy font too, so text already set in it is left as it is. Repeat for more fonts')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;normalize' {
            [CompletionResult]::new('--form', '--form', [CompletionResultType]::ParameterName, 'How far to go')
            [CompletionResult]::new('--stats', '--stats', [CompletionResultType]::ParameterName, 'Print how many times each rule changed the text to standard error, as TSV')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Write nothing; exit with status 1 if any input is not already in the form')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;type' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'How many words to show for each')
            [CompletionResult]::new('--prev', '--prev', [CompletionResultType]::ParameterName, 'The Kannada word before, which can change the order')
            [CompletionResult]::new('--scores', '--scores', [CompletionResultType]::ParameterName, 'Show 1 word per line, with its score and kind: word, spelling (not in the word list) or completion')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;completions' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;manpages' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'kannadi;help' {
            [CompletionResult]::new('to-unicode', 'to-unicode', [CompletionResultType]::ParameterValue, 'Convert documents in legacy Kannada fonts to Unicode')
            [CompletionResult]::new('to-nudi', 'to-nudi', [CompletionResultType]::ParameterValue, 'Convert Unicode Kannada documents to Nudi glyph codes')
            [CompletionResult]::new('normalize', 'normalize', [CompletionResultType]::ParameterValue, 'Normalize Unicode Kannada text, following the Kannada text normalization profile')
            [CompletionResult]::new('type', 'type', [CompletionResultType]::ParameterValue, 'Show the Kannada words for words typed in English letters, best first')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Print a shell completion script to standard output')
            [CompletionResult]::new('manpages', 'manpages', [CompletionResultType]::ParameterValue, 'Write the man pages to a folder')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'kannadi;help;to-unicode' {
            break
        }
        'kannadi;help;to-nudi' {
            break
        }
        'kannadi;help;normalize' {
            break
        }
        'kannadi;help;type' {
            break
        }
        'kannadi;help;completions' {
            break
        }
        'kannadi;help;manpages' {
            break
        }
        'kannadi;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
