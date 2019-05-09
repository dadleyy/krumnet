EXE=./dist/krumpled/api/bin/api
SRC=$(wildcard **/*.go *.go)
GO=go

all: $(EXE)

$(EXE): $(SRC)
	$(GO) build -o $(EXE) -x
