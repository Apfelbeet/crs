#!/bin/bash

git clean -df
make distclean; make functionaltest

date=$(git show -s --format=%ci HEAD | grep -Po "^.....(\K..)")
echo $date

if [ $date -le 5 ]; then
	exit 0
else
	exit 1
fi
