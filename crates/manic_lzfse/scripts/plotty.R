#!/usr/bin/env Rscript
library(ggplot2)
library(stringr)

# Plots lmdy LZ77 type LMD data as output.
# Usage: plotty FILE
# $ Rscript scripts/plotty.R ~/Desktop/lmds/alice29.txt.lmd
#
# Bash example:
# $ shopt -s nullglob; for i in ~/Desktop/lmds/*.lmd;do Rscript scripts/plotty.R "$i";done
args <- commandArgs(trailingOnly = TRUE)
file_in <- args[1]
file_ll <- paste(file_in, "ll", "jpg", sep = ".")
file_ml <- paste(file_in, "ml", "jpg", sep = ".")
file_md <- paste(file_in, "md", "jpg", sep = ".")
file_lm <- paste(file_in, "lm", "jpg", sep = ".")

# inefficient vector expansion.
ls <- numeric()
ms <- numeric()
ds <- numeric()

# import FILE
con <- file(file_in, "r")
while (TRUE) {
    line <- readLines(con, n = 1)
    if (length(line) == 0) {
        break
    }
    match <- str_match(line, regex("^([L|M|D]):\\s*(.*?$)"))
    switch(
        match[2],
        "L" = ls <- c(ls, as.numeric(match[3])),
        "M" = ms <- c(ms, min(as.numeric(match[3]), 512)),
        "D" = ds <- c(ds, min(as.numeric(match[3]), 512)),
        "NA" = cat("BAD: ", line),
    )
}
close(con)

# literal length
df <- data.frame(x = ls)
jpeg(filename = file_ll, width = 1280, height = 1024)
ggplot(df, aes(x = x)) +
    geom_histogram(
        binwidth = 2,
        color = "blue", fill = "white"
    ) +
    scale_x_continuous(name = "literal_length binwidth(2)") +
    scale_y_continuous(name = "count log2", trans = "log2") +
    ggtitle("LITERAL LENGTH", basename(file_in))
dev.off()

# match length
df <- data.frame(x = ms)
jpeg(filename = file_ml, width = 1280, height = 1024)
ggplot(df, aes(x = x)) +
    geom_histogram(
        binwidth = 4,
        color = "blue", fill = "white"
    ) +
    scale_x_continuous(name = "match_length binwidth(4), clamped(512)") +
    scale_y_continuous(name = "count log2", trans = "log2") +
    ggtitle("MATCH LENGTH", basename(file_in))
dev.off()

# match_distance
df <- data.frame(x = ds)
jpeg(filename = file_md, width = 1280, height = 1024)
ggplot(df, aes(x = x)) +
    geom_histogram(
        binwidth = 4,
        boundary = 0,
        color = "blue", fill = "white"
    ) +
    scale_x_continuous(name = "match_distance binwidth(4), clamped(512)") +
    scale_y_continuous(name = "count log2", trans = "log2") +
    ggtitle("MATCH DISTANCE", basename(file_in))
dev.off()