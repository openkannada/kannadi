# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_kannadi_global_optspecs
    string join \n h/help V/version
end

function __fish_kannadi_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_kannadi_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_kannadi_using_subcommand
    set -l cmd (__fish_kannadi_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c kannadi -n "__fish_kannadi_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c kannadi -n "__fish_kannadi_needs_command" -s V -l version -d 'Print version'
complete -c kannadi -n "__fish_kannadi_needs_command" -f -a "to-unicode" -d 'Convert documents in legacy Kannada fonts to Unicode'
complete -c kannadi -n "__fish_kannadi_needs_command" -f -a "to-nudi" -d 'Convert Unicode Kannada documents to Nudi glyph codes'
complete -c kannadi -n "__fish_kannadi_needs_command" -f -a "completions" -d 'Print a shell completion script to standard output'
complete -c kannadi -n "__fish_kannadi_needs_command" -f -a "manpages" -d 'Write the man pages to a folder'
complete -c kannadi -n "__fish_kannadi_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -s o -l out -d 'Folder to write the converted files and reports to' -r -F
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l layout -d 'Layout of text files, and of fonts added with --legacy-font that name none: nudi (Nudi and Baraha ANSI), nudi-b (Nudi B-Akshara), kan-eng, akruti or shree-lipi' -r
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l font -d 'Font to set converted text in, in HTML and DOCX files' -r
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l legacy-font -d 'Treat this font as a legacy font too, in the layout after `=` ("My Font=akruti") or else the --layout one. Repeat for more fonts' -r
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l strict -d 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l kannada-digits -d 'Write digits as Kannada digits, for text set in the k variants of the Nudi fonts'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -l keep-zeros -d 'Keep a zero typed between letters as a digit; by default it becomes ಂ'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-unicode" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-nudi" -s o -l out -d 'Folder to write the converted files and reports to' -r -F
complete -c kannadi -n "__fish_kannadi_using_subcommand to-nudi" -l font -d 'Font to set converted text in, in HTML and DOCX files' -r
complete -c kannadi -n "__fish_kannadi_using_subcommand to-nudi" -l legacy-font -d 'Treat this font as a legacy font too, so text already set in it is left as it is. Repeat for more fonts' -r
complete -c kannadi -n "__fish_kannadi_using_subcommand to-nudi" -l strict -d 'Exit with status 1 if any file has unknown glyphs, malformed syllables or unencodable characters; likely typos do not count'
complete -c kannadi -n "__fish_kannadi_using_subcommand to-nudi" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c kannadi -n "__fish_kannadi_using_subcommand completions" -s h -l help -d 'Print help'
complete -c kannadi -n "__fish_kannadi_using_subcommand manpages" -s h -l help -d 'Print help'
complete -c kannadi -n "__fish_kannadi_using_subcommand help; and not __fish_seen_subcommand_from to-unicode to-nudi completions manpages help" -f -a "to-unicode" -d 'Convert documents in legacy Kannada fonts to Unicode'
complete -c kannadi -n "__fish_kannadi_using_subcommand help; and not __fish_seen_subcommand_from to-unicode to-nudi completions manpages help" -f -a "to-nudi" -d 'Convert Unicode Kannada documents to Nudi glyph codes'
complete -c kannadi -n "__fish_kannadi_using_subcommand help; and not __fish_seen_subcommand_from to-unicode to-nudi completions manpages help" -f -a "completions" -d 'Print a shell completion script to standard output'
complete -c kannadi -n "__fish_kannadi_using_subcommand help; and not __fish_seen_subcommand_from to-unicode to-nudi completions manpages help" -f -a "manpages" -d 'Write the man pages to a folder'
complete -c kannadi -n "__fish_kannadi_using_subcommand help; and not __fish_seen_subcommand_from to-unicode to-nudi completions manpages help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
