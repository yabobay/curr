#!/usr/bin/env perl

use utf8;
binmode(STDOUT, 'utf8');

use strict; use warnings;
use feature 'say';

use Finance::Currency::Convert::ECBdaily;
use LWP::Protocol::https;
use Text::Table;

sub conv {
  my($x, $a, $b) = @_;
  return $x if $a eq $b;
  my $y = Finance::Currency::Convert::ECBdaily::convert($x, $a, $b);
  return sprintf("%.2f", $y);
}

my @currs = map uc, grep /[a-z]+/, @ARGV;
my @value = grep /\d+/, @ARGV;
die "You need to enter a currency code lol" unless @currs;
push @value, 1 unless @value;

my $tbl = Text::Table->new(@currs);

for my $v (@value) {
  my @l;
  push @l, conv($v, $currs[0], $_) for @currs;
  $tbl->load([@l]);
}

my $dashes = '-' x length join ' ', @currs;
say $dashes; print $tbl; say $dashes;

