#! /bin/sh
# ***********************************************************************
# This is pretty much straight from O'Reilly "sed & awk", 2nd Edition, p 51
#                                                                        
# Use sed to substitute one string for another.
# Specified file is modified.
#

PRG_NAME=`basename $0`
USAGE="Usage: $PRG_NAME pattern newstring file"


##########################################################
# Check number of arguments.
##########################################################
if [ $# -ne 3 ]
then
  echo "$USAGE"
  exit 1
fi

pattern=$1
newstring=$2
IN_FILE="$3"
OUT_FILE="${IN_FILE}_tmp.junk.tmp"
#Check that the file exists
if [ ! -f "$IN_FILE" ]
then
  echo "ERROR: No input file. File: $IN_FILE"
  exit 1
fi

# Create Ctl-A character to be used as separator for sed
ASEP="`echo | tr '\012' '\001' `"

sed -e "s${ASEP}${pattern}${ASEP}${newstring}${ASEP}g" "$IN_FILE" > "$OUT_FILE"
/bin/mv -f "$OUT_FILE" "$IN_FILE"
