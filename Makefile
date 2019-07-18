BIN=./dist/krumpled/api/bin
EXE=$(BIN)/krumpled
VENDOR_MANIFEST=./vendor/modules.txt
SRC=$(shell git ls-files '*.go')
GO=go
RM=rm -rf
LDFLAGS="-s -w"
BUILD_FLAGS=-x -v -ldflags $(LDFLAGS)
CYCLO_FLAGS=-over 15

.PHONY: all test clean

all: $(EXE)

clean:
	$(RM) $(BIN)
	$(RM) ./vendor

$(VENDOR_MANIFEST): go.mod go.sum
	$(GO) mod vendor

test: $(SRC)
	$(GO) get -v -u golang.org/x/lint/golint
	$(GO) get -v -u github.com/fzipp/gocyclo
	$(GO) get -v -u github.com/client9/misspell/cmd/misspell
	$(GO) vet
	misspell -error $(SRC)
	gocyclo $(CYCLO_FLAGS) $(SRC)
	$(GO) list ./... | grep -v /vendor/ | xargs -L1 golint -set_exit_status

$(EXE): $(SRC) $(VENDOR_MANIFEST)
	$(GO) build -o $(EXE) $(BUILD_FLAGS)
