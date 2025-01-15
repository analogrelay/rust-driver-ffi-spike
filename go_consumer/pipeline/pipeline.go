package pipeline

// #cgo LDFLAGS: -L${SRCDIR}/../../target/debug -l:libcosmos_driver.a
// #include "../../cosmos_driver/cosmos_driver.h"
import "C"
import (
	"encoding/json"
	"fmt"
	"iter"
	"runtime"
	"unsafe"
)

const RESULT_SUCCESS = 0
const RESULT_NONE = 1
const RESULT_INVALID_ARGUMENT = -1
const RESULT_INVALID_STRING = -2
const RESULT_INVALID_JSON = -3
const RESULT_NULL_POINTER = -4
const RESULT_MISALIGNED_POINTER = -5
const RESULT_PARTITION_DOES_NOT_EXIST = -6

func wrapString(s string) C.FfiSlice {
	return C.FfiSlice{
		data: unsafe.Pointer(unsafe.StringData(s)),
		len:  C.uintptr_t(len(s)),
	}
}

func wrapSlice[T any](s []T) C.FfiSlice {
	return C.FfiSlice{
		data: unsafe.Pointer(unsafe.SliceData(s)),
		len:  C.uintptr_t(len(s)),
	}
}

func withBytes(s C.FfiBytes, cb func([]byte)) {
	slice := []byte(unsafe.Slice((*byte)(s.data), s.len))
	cb(slice)
	C.bytes_free(s)
}

func convertError(code C.ResultCode) error {
	switch code {
	case RESULT_INVALID_ARGUMENT:
		return fmt.Errorf("invalid argument")
	case RESULT_INVALID_STRING:
		return fmt.Errorf("provided string was not UTF-8")
	case RESULT_INVALID_JSON:
		return fmt.Errorf("provided JSON was invalid")
	case RESULT_NULL_POINTER:
		return fmt.Errorf("a null pointer was provided")
	case RESULT_MISALIGNED_POINTER:
		return fmt.Errorf("a misaligned pointer was provided")
	case RESULT_PARTITION_DOES_NOT_EXIST:
		return fmt.Errorf("the partition does not exist")
	default:
		return fmt.Errorf("unknown error: %d", code)
	}
}

type Pipeline[T any] struct {
	ptr *C.Pipeline
}

// Creates a new pipeline with the given partition ids.
func NewPipeline[T any](partition_ids []string) (*Pipeline[T], error) {
	c_partition_ids := make([]C.FfiSlice, len(partition_ids))
	for i, partition_id := range partition_ids {
		s := wrapString(partition_id)
		c_partition_ids[i] = s
		defer runtime.KeepAlive(s)
	}

	result := C.pipeline_new(wrapSlice(c_partition_ids))
	if result.code != RESULT_SUCCESS {
		return nil, convertError(result.code)
	}

	return &Pipeline[T]{ptr: result.value}, nil
}

// Enqueues data to the given partition.
func (p *Pipeline[T]) EnqueueData(partition_id string, data []byte) error {
	c_partition_id := wrapString(partition_id)
	defer runtime.KeepAlive(c_partition_id)

	c_buffer := wrapSlice(data)
	defer runtime.KeepAlive(c_buffer)

	result := C.pipeline_enqueue_data_raw(p.ptr, c_partition_id, c_buffer)
	if result != RESULT_SUCCESS {
		return convertError(result)
	}
	return nil
}

func (p *Pipeline[T]) Iter() iter.Seq2[*T, error] {
	return func(yield func(*T, error) bool) {
		for val, err, ok := p.Next(); ok; val, err, ok = p.Next() {
			result := yield(val, err)
			if !result {
				// Bail on break
				return
			}
			if err != nil {
				// Also bail on errors
				return
			}
		}
	}
}

func (p *Pipeline[T]) Next() (*T, error, bool) {
	result := C.pipeline_next_item_raw(p.ptr)
	switch result.code {
	case RESULT_SUCCESS:
		var t T
		var err error
		withBytes(result.value, func(bytes []byte) {
			err = json.Unmarshal(bytes, &t)
		})
		return &t, err, true
	case RESULT_NONE:
		return nil, nil, false
	default:
		return nil, convertError(result.code), true
	}
}

func (p *Pipeline[T]) Free() {
	if p.ptr != nil {
		C.pipeline_free(p.ptr)
		p.ptr = nil
	}
}
