#!/usr/bin/perl -p
# c89filter.pl < mrustc.c > out.c: a crude, spike-only text rewrite of mrustc's Msvc-flavour C
# toward what Visual C++ 4.1 takes, to see what cl says about the rest once the known
# blockers (found by probes/) are out of the way. Not a design for the real C89 mode, which
# belongs in mrustc's generator.
s/\bstatic inline\b/static __inline/g;
s/__declspec\((noreturn|selectany|align\(\d+\))\)\s*//g;
$_ = '' if /^typedef char alignof_assert_/;
s/#define ALIGNOF\(t\) __alignof\(t\)/#define ALIGNOF(t) 8/;
# 64-bit literals: ull -> ui64, ll -> i64 (outside comments, roughly)
my ($code, $comment) = /^(.*?)(\/\/.*)?$/s ? ($1, $2 // '') : ($_, '');
$code =~ s/\b(0x[0-9A-Fa-f]+|\d+)ull\b/$1ui64/g;
$code =~ s/\b(0x[0-9A-Fa-f]+|\d+)ll\b/$1i64/g;
# statics: union { T val; X raw[N]; } x = { .raw = {...} } -> raw first, positional
if ($code =~ /^(extern |static )?union u_static_/) {
    $code =~ s/\{ (.*?) val; (\w+) raw\[(\d+)\]; \}/{ $2 raw[$3]; $1 val; }/;
    $code =~ s/= \{ \.raw = /= { /;
}
# [x; N] in generated code: "for(unsigned int i = 0; ...)" and one statement on the next line
our $close_next;
if ($close_next && $code =~ /;\s*$/) { $code =~ s/;\s*$/; }/; $close_next = 0; }
if ($code =~ s/^(\t+)for\(unsigned int i = 0;/$1\{ unsigned int i; for(i = 0;/) { $close_next = 1; }
# a static declared after statements (#[track_caller] without a caller)
$code =~ s/^(\t+)(static struct \w+ mrustc_empty_caller_location = .*;)\s*$/$1\{ $2 }/;
$_ = $code . $comment;
$_ .= "\n" unless /\n\z/;
